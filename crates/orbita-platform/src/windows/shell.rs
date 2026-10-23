//! Item "Converter com Órbita" no menu do botão direito do Explorer.
//!
//! Usa o modelo "DelegateExecute" da Microsoft: o verbo aponta para uma classe
//! COM servida pelo próprio orbita.exe (LocalServer32). Assim o Explorer entrega
//! a seleção inteira de uma vez, sem limite de 15 arquivos, sem abrir um
//! processo por arquivo e sem carregar nenhuma DLL dentro do Explorer.
//! Tudo fica em HKEY_CURRENT_USER: não precisa de administrador.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ::windows::core::{implement, Interface, Ref, BOOL, GUID, HSTRING, IUnknown, PCWSTR, PWSTR};
use ::windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_FAIL, E_NOINTERFACE, POINT};
use ::windows::Win32::System::Com::StructuredStorage::IPropertyBag;
use ::windows::Win32::System::Com::{
    CoInitializeEx, CoRegisterClassObject, CoTaskMemFree, IClassFactory, IClassFactory_Impl,
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, REGCLS_MULTIPLEUSE,
};
use ::windows::Win32::System::Ole::{IObjectWithSite, IObjectWithSite_Impl};
use ::windows::Win32::System::Registry::{
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use ::windows::Win32::UI::Shell::{
    IExecuteCommand, IExecuteCommand_Impl, IInitializeCommand, IInitializeCommand_Impl,
    IObjectWithSelection, IObjectWithSelection_Impl, IShellItemArray, SHChangeNotify,
    SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SIGDN_FILESYSPATH,
};
use ::windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG};

use crate::{Invocation, PlatformError, Result};

/// Identificador fixo da classe COM do Órbita.
const CLSID: GUID = GUID::from_u128(0x6c9b2e8a_4f1d_4b7e_9a3c_0d5e8f7a1b2c);
const CLSID_STR: &str = "{6C9B2E8A-4F1D-4B7E-9A3C-0D5E8F7A1B2C}";
const VERB: &str = "Orbita";
const VERB_TEXT: &str = "Converter com Órbita";

type Callback = Box<dyn Fn(Invocation) + Send + Sync>;

// ---------------------------------------------------------------------------
// Registro
// ---------------------------------------------------------------------------

fn classes(sub: &str) -> String {
    format!(r"Software\Classes\{sub}")
}

pub(crate) fn set_value(key: &str, name: Option<&str>, value: &str) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .ok()
        .map_err(|e| PlatformError::Os(format!("registro {key}: {e}")))?;

        let wide: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let bytes = std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2);
        let name = name.map(HSTRING::from);
        let result = RegSetValueExW(
            hkey,
            name.as_ref().map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr())),
            None,
            REG_SZ,
            Some(bytes),
        );
        let _ = ::windows::Win32::System::Registry::RegCloseKey(hkey);
        result.ok().map_err(|e| PlatformError::Os(format!("registro {key}: {e}")))
    }
}

fn delete_tree(key: &str) {
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(key));
    }
}

fn notify_shell() {
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

pub fn register(exe: &Path, extensions: &[&str]) -> Result<()> {
    let exe = exe.display().to_string();
    let clsid = classes(&format!(r"CLSID\{CLSID_STR}"));
    set_value(&clsid, None, "Órbita")?;
    set_value(&format!(r"{clsid}\LocalServer32"), None, &format!("\"{exe}\""))?;

    for ext in extensions {
        let verb = classes(&format!(r"SystemFileAssociations\.{ext}\shell\{VERB}"));
        set_value(&verb, Some("MUIVerb"), VERB_TEXT)?;
        set_value(&verb, Some("Icon"), &format!("\"{exe}\",0"))?;
        set_value(&verb, Some("MultiSelectModel"), "Player")?;
        set_value(&format!(r"{verb}\command"), Some("DelegateExecute"), CLSID_STR)?;
    }
    notify_shell();
    Ok(())
}

pub fn unregister(extensions: &[&str]) -> Result<()> {
    for ext in extensions {
        delete_tree(&classes(&format!(r"SystemFileAssociations\.{ext}\shell\{VERB}")));
    }
    delete_tree(&classes(&format!(r"CLSID\{CLSID_STR}")));
    notify_shell();
    Ok(())
}

// ---------------------------------------------------------------------------
// Servidor COM
// ---------------------------------------------------------------------------

/// Registra a classe COM e fica atendendo o Explorer numa thread própria.
pub fn serve(on_invoke: Callback) -> Result<()> {
    let callback: Arc<Callback> = Arc::new(on_invoke);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();

    std::thread::Builder::new()
        .name("orbita-shell".into())
        .spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let factory: IClassFactory = Factory { callback }.into();
            let registered = CoRegisterClassObject(&CLSID, &factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE);
            let _ = ready_tx.send(registered.map(|_| ()).map_err(|e| e.to_string()));
            // Laço de mensagens: o COM entrega as chamadas por aqui (STA).
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .map_err(|e| PlatformError::Os(e.to_string()))?;

    ready_rx
        .recv()
        .map_err(|e| PlatformError::Os(e.to_string()))?
        .map_err(PlatformError::Os)
}

#[implement(IClassFactory)]
struct Factory {
    callback: Arc<Callback>,
}

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, outer: Ref<IUnknown>, riid: *const GUID, object: *mut *mut core::ffi::c_void) -> ::windows::core::Result<()> {
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let command: IUnknown = ExecuteCommand {
            callback: self.callback.clone(),
            selection: RefCell::new(None),
            site: RefCell::new(None),
            point: Cell::new(None),
        }
        .into();
        unsafe { command.query(riid, object).ok() }
    }

    fn LockServer(&self, _lock: BOOL) -> ::windows::core::Result<()> {
        Ok(())
    }
}

#[implement(IExecuteCommand, IObjectWithSelection, IInitializeCommand, IObjectWithSite)]
struct ExecuteCommand {
    callback: Arc<Callback>,
    selection: RefCell<Option<IShellItemArray>>,
    site: RefCell<Option<IUnknown>>,
    point: Cell<Option<(i32, i32)>>,
}

impl IExecuteCommand_Impl for ExecuteCommand_Impl {
    fn SetKeyState(&self, _state: u32) -> ::windows::core::Result<()> {
        Ok(())
    }
    fn SetParameters(&self, _params: &PCWSTR) -> ::windows::core::Result<()> {
        Ok(())
    }
    fn SetPosition(&self, pt: &POINT) -> ::windows::core::Result<()> {
        self.point.set(Some((pt.x, pt.y)));
        Ok(())
    }
    fn SetShowWindow(&self, _show: i32) -> ::windows::core::Result<()> {
        Ok(())
    }
    fn SetNoShowUI(&self, _no_ui: BOOL) -> ::windows::core::Result<()> {
        Ok(())
    }
    fn SetDirectory(&self, _dir: &PCWSTR) -> ::windows::core::Result<()> {
        Ok(())
    }

    /// Chamado pelo Explorer quando o usuário clica no item. Só repassa os
    /// arquivos e retorna na hora; o app faz o resto em outra thread.
    fn Execute(&self) -> ::windows::core::Result<()> {
        let files = match self.selection.borrow().as_ref() {
            Some(items) => unsafe { paths(items) },
            None => return Err(E_FAIL.into()),
        };
        if !files.is_empty() {
            (self.callback)(Invocation { files, point: self.point.get() });
        }
        Ok(())
    }
}

impl IObjectWithSelection_Impl for ExecuteCommand_Impl {
    fn SetSelection(&self, items: Ref<IShellItemArray>) -> ::windows::core::Result<()> {
        *self.selection.borrow_mut() = items.cloned();
        Ok(())
    }
    fn GetSelection(&self, riid: *const GUID, out: *mut *mut core::ffi::c_void) -> ::windows::core::Result<()> {
        match self.selection.borrow().as_ref() {
            Some(items) => unsafe { items.query(riid, out).ok() },
            None => Err(E_NOINTERFACE.into()),
        }
    }
}

impl IInitializeCommand_Impl for ExecuteCommand_Impl {
    fn Initialize(&self, _name: &PCWSTR, _bag: Ref<IPropertyBag>) -> ::windows::core::Result<()> {
        Ok(())
    }
}

impl IObjectWithSite_Impl for ExecuteCommand_Impl {
    fn SetSite(&self, site: Ref<IUnknown>) -> ::windows::core::Result<()> {
        *self.site.borrow_mut() = site.cloned();
        Ok(())
    }
    fn GetSite(&self, riid: *const GUID, out: *mut *mut core::ffi::c_void) -> ::windows::core::Result<()> {
        match self.site.borrow().as_ref() {
            Some(site) => unsafe { site.query(riid, out).ok() },
            None => Err(E_FAIL.into()),
        }
    }
}

unsafe fn paths(items: &IShellItemArray) -> Vec<PathBuf> {
    let count = items.GetCount().unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let item = items.GetItemAt(i).ok()?;
            let name: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
            let path = super::pwstr_to_path(PCWSTR(name.0));
            CoTaskMemFree(Some(name.0 as *const _));
            path
        })
        .collect()
}
