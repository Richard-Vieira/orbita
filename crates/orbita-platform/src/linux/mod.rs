//! Linux: o menu é acionado pelos gerenciadores de arquivos (Nautilus, Dolphin,
//! Nemo, Thunar e o "Abrir com" de todos), que passam os caminhos como
//! argumentos; a instância única do app repassa esses argumentos para o
//! Órbita que já estiver aberto.
//!
//! Sem captura de tela: o menu usa o fundo sólido. Sem seleção do gerenciador
//! de arquivos: não há um jeito comum a todos de lê-la.

use std::path::{Path, PathBuf};

use crate::{Capture, PlatformError, Result};

pub mod autostart;
pub mod shell;

/// Ícone usado pelos itens de menu (copiado para a pasta de ícones do usuário,
/// já que o AppImage não instala nada no sistema).
const ICON_PNG: &[u8] = include_bytes!("../../../../src-tauri/icons/128x128.png");
const ICON_NAME: &str = "orbita";

pub fn selected_files() -> Result<Vec<PathBuf>> {
    Err(PlatformError::Unsupported)
}

/// O app usa a posição do mouse do próprio Tauri (X11/XWayland).
pub fn cursor_position() -> Option<(i32, i32)> {
    None
}

pub fn capture_screen(_x: i32, _y: i32, _width: u32, _height: u32) -> Option<Capture> {
    None
}

/// Abre a pasta com o arquivo selecionado (pelo D-Bus padrão dos gerenciadores
/// de arquivos); se não der, só abre a pasta.
pub fn reveal(path: &Path) -> Result<()> {
    let uri = file_uri(path);
    let shown = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.FileManager1",
            "--object-path",
            "/org/freedesktop/FileManager1",
            "--method",
            "org.freedesktop.FileManager1.ShowItems",
            &format!("['{}']", uri.replace('\'', "%27")),
            "",
        ])
        .output()
        .is_ok_and(|o| o.status.success());
    if shown {
        return Ok(());
    }
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

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

/// Caminho que continua valendo depois que o app fecha: o AppImage roda de uma
/// pasta temporária, então usamos o arquivo .AppImage em si.
pub(crate) fn stable_exe(exe: &Path) -> PathBuf {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|| exe.to_path_buf())
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PlatformError::Os("variável HOME não definida".into()))
}

/// `~/.local/share` (ou `$XDG_DATA_HOME`).
pub(crate) fn data_home() -> Result<PathBuf> {
    xdg_dir("XDG_DATA_HOME", ".local/share")
}

/// `~/.config` (ou `$XDG_CONFIG_HOME`).
pub(crate) fn config_home() -> Result<PathBuf> {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

fn xdg_dir(var: &str, fallback: &str) -> Result<PathBuf> {
    match std::env::var_os(var).map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => Ok(dir),
        _ => Ok(home()?.join(fallback)),
    }
}

/// Escreve o arquivo criando as pastas; `executable` liga a permissão de execução.
pub(crate) fn write_file(path: &Path, contents: &[u8], executable: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let os = |e: std::io::Error| PlatformError::Os(format!("{}: {e}", path.display()));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(os)?;
    }
    std::fs::write(path, contents).map_err(os)?;
    let mode = if executable { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(os)
}

/// Remove o arquivo se existir.
pub(crate) fn remove_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Caminho entre aspas para a linha `Exec=` de um arquivo .desktop.
pub(crate) fn desktop_exec_arg(path: &Path) -> String {
    let mut quoted = String::from("\"");
    for c in path.display().to_string().chars() {
        match c {
            // Escape do Exec, e a barra ainda é escapada de novo pelo formato .desktop.
            '\\' => quoted.push_str("\\\\\\\\"),
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(c);
            }
            '%' => quoted.push_str("%%"),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// Caminho entre aspas simples para o shell.
pub(crate) fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for b in path.display().to_string().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                uri.push(b as char)
            }
            _ => uri.push_str(&format!("%{b:02X}")),
        }
    }
    uri
}

pub(crate) fn install_icon() -> Result<()> {
    write_file(&icon_path()?, ICON_PNG, false)
}

pub(crate) fn remove_icon() {
    if let Ok(path) = icon_path() {
        remove_file(&path);
    }
}

fn icon_path() -> Result<PathBuf> {
    Ok(data_home()?.join(format!("icons/hicolor/128x128/apps/{ICON_NAME}.png")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_exec_paths() {
        assert_eq!(
            desktop_exec_arg(Path::new("/usr/bin/orbita")),
            "\"/usr/bin/orbita\""
        );
        assert_eq!(
            desktop_exec_arg(Path::new("/a b/100%/x$y")),
            "\"/a b/100%%/x\\\\$y\""
        );
        assert_eq!(shell_quote(Path::new("/it's here")), r"'/it'\''s here'");
    }

    #[test]
    fn builds_file_uris() {
        assert_eq!(
            file_uri(Path::new("/home/a/Área de trabalho/x.pdf")),
            "file:///home/a/%C3%81rea%20de%20trabalho/x.pdf"
        );
    }
}
