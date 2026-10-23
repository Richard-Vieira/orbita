//! Iniciar com o sistema: `~/.config/autostart/orbita.desktop` (padrão XDG,
//! usado por GNOME, KDE, Cinnamon, XFCE...).

use std::path::{Path, PathBuf};

use super::{config_home, desktop_exec_arg, remove_file, stable_exe, write_file};
use crate::Result;

fn file() -> Result<PathBuf> {
    Ok(config_home()?.join("autostart/orbita.desktop"))
}

pub fn set(name: &str, exe: &Path, enabled: bool) -> Result<()> {
    let path = file()?;
    if !enabled {
        remove_file(&path);
        return Ok(());
    }
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={name}\n\
         Exec={exec} --background\n\
         Icon={icon}\n\
         Terminal=false\n\
         NoDisplay=true\n\
         X-GNOME-Autostart-enabled=true\n",
        exec = desktop_exec_arg(&stable_exe(exe)),
        icon = super::ICON_NAME,
    );
    write_file(&path, entry.as_bytes(), false)
}

pub fn is_enabled(_name: &str) -> bool {
    file().is_ok_and(|p| p.exists())
}
