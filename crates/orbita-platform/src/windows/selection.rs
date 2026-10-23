//! Arquivos selecionados na janela do Explorer em primeiro plano ou na área de
//! trabalho (usado pelo atalho de teclado).
//!
//! No Windows 11 várias abas compartilham a mesma janela; para pegar a aba
//! certa comparamos a janela da aba ativa (ShellTabWindowClass) com a de cada
//! navegador registrado em IShellWindows.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use ::windows::core::{Interface, Result as WinResult, PWSTR};
use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED,
};
use ::windows::Win32::System::Variant::VARIANT;
use ::windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, SID_STopLevelBrowser,
    ShellWindows, CSIDL_DESKTOP, SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use ::windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetClassNameW, GetForegroundWindow};
use ::windows::core::w;

use crate::Result;

/// Tempo máximo esperando o Explorer responder (ele pode estar ocupado).
const TIMEOUT: Duration = Duration::from_secs(3);

pub fn selected_files() -> Result<Vec<PathBuf>> {
    let foreground = unsafe { GetForegroundWindow() };
    let hwnd = foreground.0 as isize;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = query(HWND(hwnd as *mut _)).unwrap_or_default();
        let _ = tx.send(result);
        CoUninitialize();
    });
    Ok(rx.recv_timeout(TIMEOUT).unwrap_or_default())
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

unsafe fn query(foreground: HWND) -> WinResult<Vec<PathBuf>> {
    let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;
    let browser = match class_name(foreground).as_str() {
        "Progman" | "WorkerW" => desktop_browser(&windows)?,
        "CabinetWClass" | "ExploreWClass" => match explorer_browser(&windows, foreground)? {
            Some(b) => b,
            None => return Ok(Vec::new()),
        },
        _ => return Ok(Vec::new()),
    };
    let view = browser.QueryActiveShellView()?;
    let folder_view: IFolderView = view.cast()?;
    let items: IShellItemArray = match folder_view.Items(SVGIO_SELECTION) {
        Ok(items) => items,
        Err(_) => return Ok(Vec::new()), // nada selecionado
    };
    Ok(paths(&items))
}

unsafe fn desktop_browser(windows: &IShellWindows) -> WinResult<IShellBrowser> {
    let location = VARIANT::from(CSIDL_DESKTOP as i32);
    let root = VARIANT::default();
    let mut hwnd = 0i32;
    let dispatch = windows.FindWindowSW(&location, &root, SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)?;
    let provider: IServiceProvider = dispatch.cast()?;
    provider.QueryService(&SID_STopLevelBrowser)
}

unsafe fn explorer_browser(windows: &IShellWindows, frame: HWND) -> WinResult<Option<IShellBrowser>> {
    // Aba ativa: a primeira ShellTabWindowClass filha da janela (ordem Z).
    let active_tab = FindWindowExW(Some(frame), None, w!("ShellTabWindowClass"), None).ok();
    let mut fallback = None;

    for i in 0..windows.Count()? {
        let Ok(dispatch) = windows.Item(&VARIANT::from(i)) else { continue };
        let Ok(provider) = dispatch.cast::<IServiceProvider>() else { continue };
        let Ok(browser) = provider.QueryService::<IShellBrowser>(&SID_STopLevelBrowser) else { continue };
        let window = browser.GetWindow().unwrap_or_default();

        if active_tab.is_some_and(|tab| tab == window) {
            return Ok(Some(browser));
        }
        let same_frame = window == frame
            || dispatch
                .cast::<IWebBrowserApp>()
                .and_then(|app| app.HWND())
                .is_ok_and(|h| h.0 == frame.0 as isize);
        if same_frame && fallback.is_none() {
            fallback = Some(browser);
        }
    }
    Ok(fallback)
}

unsafe fn paths(items: &IShellItemArray) -> Vec<PathBuf> {
    let count = items.GetCount().unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let item = items.GetItemAt(i).ok()?;
            let name: PWSTR = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
            let path = super::pwstr_to_path(::windows::core::PCWSTR(name.0));
            CoTaskMemFree(Some(name.0 as *const _));
            path
        })
        .filter(|p| p.is_file())
        .collect()
}
