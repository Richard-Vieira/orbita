//! Linux: ainda não implementado.
//!
//! Plano: o menu será acionado por ações dos gerenciadores de arquivos
//! (Nautilus, Dolphin, Nemo, Thunar), que passam os caminhos como argumentos;
//! a captura de fundo usará X11 e, no Wayland, o fundo ficará sólido.

use std::path::{Path, PathBuf};

use crate::{Capture, Invocation, PlatformError, Result};

pub mod shell {
    use super::*;

    pub fn register(_exe: &Path, _extensions: &[&str]) -> Result<()> {
        Err(PlatformError::Unsupported)
    }

    pub fn unregister(_extensions: &[&str]) -> Result<()> {
        Err(PlatformError::Unsupported)
    }

    pub fn serve(_on_invoke: Box<dyn Fn(Invocation) + Send + Sync>) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}

pub mod autostart {
    use super::*;

    pub fn set(_name: &str, _exe: &Path, _enabled: bool) -> Result<()> {
        Err(PlatformError::Unsupported)
    }

    pub fn is_enabled(_name: &str) -> bool {
        false
    }
}

pub fn selected_files() -> Result<Vec<PathBuf>> {
    Err(PlatformError::Unsupported)
}

pub fn cursor_position() -> Option<(i32, i32)> {
    None
}

pub fn capture_screen(_x: i32, _y: i32, _width: u32, _height: u32) -> Option<Capture> {
    None
}

pub fn reveal(path: &Path) -> Result<()> {
    let dir = path.parent().unwrap_or(path);
    std::process::Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| PlatformError::Os(e.to_string()))
}

pub fn show_without_focus(_handle: isize) -> bool {
    false
}

pub fn transparency_enabled() -> bool {
    true
}
