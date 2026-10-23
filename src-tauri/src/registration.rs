//! Registro no sistema: item do menu do botão direito e início automático.

use orbita_core::Format;
use orbita_platform::{autostart, shell};

/// Nome da entrada de início automático. Precisa ser igual ao `productName`
/// do tauri.conf.json, porque o desinstalador do Tauri remove esse nome.
pub const AUTOSTART_NAME: &str = "Órbita";

fn exe() -> Result<std::path::PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())
}

/// Garante que o item do menu de contexto aponta para este executável.
pub fn ensure_shell_integration() -> Result<(), String> {
    shell::register(&exe()?, Format::EXTENSIONS).map_err(|e| e.to_string())
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    autostart::set(AUTOSTART_NAME, &exe()?, enabled).map_err(|e| e.to_string())
}

pub fn autostart_enabled() -> bool {
    autostart::is_enabled(AUTOSTART_NAME)
}

/// `orbita --register` (instalador).
pub fn install() -> Result<(), String> {
    ensure_shell_integration()?;
    set_autostart(true)
}

/// `orbita --unregister` (desinstalador): remove tudo que o app criou no registro.
pub fn uninstall() -> Result<(), String> {
    shell::unregister(Format::EXTENSIONS).map_err(|e| e.to_string())?;
    set_autostart(false)
}
