//! Iniciar com o Windows (chave Run do usuário atual).

use std::path::Path;

use ::windows::core::HSTRING;
use ::windows::Win32::System::Registry::{RegDeleteKeyValueW, RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

use crate::Result;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn set(name: &str, exe: &Path, enabled: bool) -> Result<()> {
    if enabled {
        super::shell::set_value(RUN_KEY, Some(name), &format!("\"{}\" --background", exe.display()))
    } else {
        unsafe {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(RUN_KEY), &HSTRING::from(name));
        }
        Ok(())
    }
}

pub fn is_enabled(name: &str) -> bool {
    let mut size = 0u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(RUN_KEY),
            &HSTRING::from(name),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .is_ok()
    }
}
