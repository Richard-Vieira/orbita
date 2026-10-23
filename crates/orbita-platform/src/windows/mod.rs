//! Implementação para Windows.

use std::path::{Path, PathBuf};

use ::windows::core::{HSTRING, PCWSTR};
use ::windows::Win32::Foundation::POINT;
use ::windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use ::windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use ::windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems};
use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
};

use crate::{PlatformError, Result};

pub mod autostart;
mod capture;
mod selection;
pub mod shell;

pub use capture::capture_screen;
pub use selection::selected_files;

pub fn cursor_position() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

pub fn reveal(path: &Path) -> Result<()> {
    let path = path.to_path_buf();
    // Roda numa thread com COM inicializado; o Explorer cuida do resto.
    std::thread::spawn(move || unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let pidl = ILCreateFromPathW(&HSTRING::from(path.as_os_str()));
        if !pidl.is_null() {
            let _ = SHOpenFolderAndSelectItems(pidl, None, 0);
            ILFree(Some(pidl));
        }
        CoUninitialize();
    })
    .join()
    .map_err(|_| PlatformError::Os("falha ao abrir a pasta".into()))
}

pub fn show_without_focus(handle: isize) -> bool {
    let hwnd = HWND(handle as *mut _);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW).is_ok()
    }
}

pub fn transparency_enabled() -> bool {
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            &HSTRING::from("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    status.is_err() || value != 0
}

/// Converte uma string de largura variável do Windows em `PathBuf`.
pub(crate) fn pwstr_to_path(p: PCWSTR) -> Option<PathBuf> {
    if p.is_null() {
        return None;
    }
    let s = unsafe { p.to_string().ok()? };
    (!s.is_empty()).then(|| PathBuf::from(s))
}
