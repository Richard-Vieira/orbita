//! Item "Converter com Órbita" nos gerenciadores de arquivos. Tudo fica na
//! pasta do usuário (sem sudo):
//!
//! - "Abrir com" (todos): `~/.local/share/applications/orbita-converter.desktop`
//! - Nautilus (GNOME, Ubuntu): botão direito › Scripts › Converter com Órbita
//! - Dolphin (KDE): menu de serviço, plasma 5 e 6
//! - Nemo (Cinnamon, Mint): ação do Nemo
//! - Thunar (XFCE): ação personalizada no `uca.xml`
//!
//! Cada gerenciador passa os arquivos como argumentos; o app aberto recebe os
//! argumentos pela instância única.

use std::path::{Path, PathBuf};

use super::{
    config_home, data_home, desktop_exec_arg, remove_file, shell_quote, stable_exe, write_file,
};
use crate::{Invocation, Result};

const NAME: &str = "Converter com Órbita";
const COMMENT: &str = "Converter PDFs e imagens num menu radial";
/// Identifica o que é do Órbita (arquivos e a ação do Thunar).
const ID: &str = "orbita-converter";

pub fn register(exe: &Path, extensions: &[&str]) -> Result<()> {
    let exe = stable_exe(exe);
    let exec = desktop_exec_arg(&exe);
    let mimes = mime_types(extensions);
    let data = data_home()?;

    super::install_icon()?;

    write_file(
        &open_with_file(&data),
        open_with_entry(&exec, &mimes).as_bytes(),
        false,
    )?;
    // Atualiza o cache do "Abrir com" (se a ferramenta existir).
    let _ = std::process::Command::new("update-desktop-database")
        .arg(data.join("applications"))
        .output();

    write_file(
        &nautilus_script(&data),
        nautilus_contents(&exe).as_bytes(),
        true,
    )?;
    for file in dolphin_files(&data) {
        write_file(&file, dolphin_entry(&exec, &mimes).as_bytes(), true)?;
    }
    write_file(
        &nemo_file(&data),
        nemo_entry(&exe, extensions).as_bytes(),
        false,
    )?;
    // O Thunar é o único que guarda as ações num arquivo compartilhado: se ele
    // não puder ser lido, os outros continuam funcionando.
    if let Err(e) = thunar_register(&exe, extensions) {
        log::warn!("Thunar: {e}");
    }
    Ok(())
}

pub fn unregister(_extensions: &[&str]) -> Result<()> {
    let data = data_home()?;
    remove_file(&open_with_file(&data));
    let _ = std::process::Command::new("update-desktop-database")
        .arg(data.join("applications"))
        .output();
    remove_file(&nautilus_script(&data));
    for file in dolphin_files(&data) {
        remove_file(&file);
    }
    remove_file(&nemo_file(&data));
    let _ = thunar_unregister();
    super::remove_icon();
    Ok(())
}

/// Nada para escutar: os pedidos chegam como argumentos de linha de comando.
pub fn serve(_on_invoke: Box<dyn Fn(Invocation) + Send + Sync>) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Arquivos de cada gerenciador
// ---------------------------------------------------------------------------

fn open_with_file(data: &Path) -> PathBuf {
    data.join(format!("applications/{ID}.desktop"))
}

fn nautilus_script(data: &Path) -> PathBuf {
    data.join("nautilus/scripts").join(NAME)
}

fn dolphin_files(data: &Path) -> [PathBuf; 2] {
    [
        data.join(format!("kio/servicemenus/{ID}.desktop")), // Plasma 6
        data.join(format!("kservices5/ServiceMenus/{ID}.desktop")), // Plasma 5
    ]
}

fn nemo_file(data: &Path) -> PathBuf {
    data.join(format!("nemo/actions/{ID}.nemo_action"))
}

fn thunar_file() -> Result<PathBuf> {
    Ok(config_home()?.join("Thunar/uca.xml"))
}

fn open_with_entry(exec: &str, mimes: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={NAME}\n\
         Comment={COMMENT}\n\
         Icon={icon}\n\
         Exec={exec} %F\n\
         MimeType={mimes}\n\
         Terminal=false\n\
         Categories=Utility;\n",
        icon = super::ICON_NAME,
    )
}

fn nautilus_contents(exe: &Path) -> String {
    // O Nautilus passa os arquivos selecionados como argumentos.
    format!(
        "#!/bin/sh\n# Criado pelo Órbita. Para remover: orbita --unregister\nexec {} \"$@\"\n",
        shell_quote(exe)
    )
}

fn dolphin_entry(exec: &str, mimes: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Service\n\
         X-KDE-ServiceTypes=KonqPopupMenu/Plugin\n\
         MimeType={mimes}\n\
         Actions=orbita\n\
         X-KDE-Priority=TopLevel\n\
         \n\
         [Desktop Action orbita]\n\
         Name={NAME}\n\
         Icon={icon}\n\
         Exec={exec} %F\n",
        icon = super::ICON_NAME,
    )
}

fn nemo_entry(exe: &Path, extensions: &[&str]) -> String {
    format!(
        "[Nemo Action]\n\
         Name={NAME}\n\
         Comment={COMMENT}\n\
         Exec={exe} %F\n\
         Icon-Name={icon}\n\
         Selection=notnone\n\
         Extensions={exts};\n",
        exe = shell_quote(exe),
        icon = super::ICON_NAME,
        exts = extensions.join(";"),
    )
}

fn thunar_action(exe: &Path, extensions: &[&str]) -> String {
    let patterns: Vec<String> = extensions
        .iter()
        .flat_map(|e| [format!("*.{e}"), format!("*.{}", e.to_ascii_uppercase())])
        .collect();
    format!(
        "<action>\n\
         \t<icon>{icon}</icon>\n\
         \t<name>{NAME}</name>\n\
         \t<submenu></submenu>\n\
         \t<unique-id>{ID}</unique-id>\n\
         \t<command>{command} %F</command>\n\
         \t<description>{COMMENT}</description>\n\
         \t<range>*</range>\n\
         \t<patterns>{patterns}</patterns>\n\
         \t<image-files/>\n\
         \t<other-files/>\n\
         </action>\n",
        icon = super::ICON_NAME,
        command = xml_escape(&shell_quote(exe)),
        patterns = patterns.join(";"),
    )
}

// ---------------------------------------------------------------------------
// Thunar (uca.xml)
// ---------------------------------------------------------------------------

fn thunar_register(exe: &Path, extensions: &[&str]) -> Result<()> {
    let path = thunar_file()?;
    let action = thunar_action(exe, extensions);
    let contents = match std::fs::read_to_string(&path) {
        Ok(current) => {
            let current = without_thunar_action(&current);
            let Some(end) = current.rfind("</actions>") else {
                return Err(crate::PlatformError::Os(format!(
                    "{} em formato inesperado",
                    path.display()
                )));
            };
            format!("{}{action}{}", &current[..end], &current[end..])
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n{action}</actions>\n")
        }
        Err(e) => return Err(crate::PlatformError::Os(format!("{}: {e}", path.display()))),
    };
    write_file(&path, contents.as_bytes(), false)
}

fn thunar_unregister() -> Result<()> {
    let path = thunar_file()?;
    let Ok(current) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let cleaned = without_thunar_action(&current);
    if cleaned != current {
        write_file(&path, cleaned.as_bytes(), false)?;
    }
    Ok(())
}

/// Tira do `uca.xml` a ação do Órbita (achada pelo unique-id), deixando as outras.
fn without_thunar_action(xml: &str) -> String {
    let marker = format!("<unique-id>{ID}</unique-id>");
    let Some(at) = xml.find(&marker) else {
        return xml.to_string();
    };
    let (Some(start), Some(end)) = (xml[..at].rfind("<action>"), xml[at..].find("</action>"))
    else {
        return xml.to_string();
    };
    let mut end = at + end + "</action>".len();
    if xml[end..].starts_with('\n') {
        end += 1;
    }
    format!("{}{}", &xml[..start], &xml[end..])
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Tipos MIME das extensões, no formato `a;b;` do .desktop.
fn mime_types(extensions: &[&str]) -> String {
    let mut mimes: Vec<&str> = Vec::new();
    for ext in extensions {
        let mime = match *ext {
            "pdf" => "application/pdf",
            "png" => "image/png",
            "jpg" | "jpeg" | "jfif" => "image/jpeg",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "gif" => "image/gif",
            "tif" | "tiff" => "image/tiff",
            _ => continue,
        };
        if !mimes.contains(&mime) {
            mimes.push(mime);
        }
    }
    mimes.iter().map(|m| format!("{m};")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXTS: &[&str] = &["pdf", "png", "jpg", "jpeg"];

    #[test]
    fn mime_list_has_no_duplicates() {
        assert_eq!(mime_types(EXTS), "application/pdf;image/png;image/jpeg;");
    }

    #[test]
    fn thunar_action_is_replaced_and_removed() {
        let other = "<action>\n\t<name>Outra</name>\n\t<unique-id>123</unique-id>\n</action>\n";
        let base =
            format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n{other}</actions>\n");
        let ours = thunar_action(Path::new("/usr/bin/orbita"), EXTS);
        let with_ours = base.replace("</actions>", &format!("{ours}</actions>"));
        assert_eq!(without_thunar_action(&with_ours), base);
        assert_eq!(without_thunar_action(&base), base);
        assert!(ours.contains("<command>'/usr/bin/orbita' %F</command>"));
        assert!(ours.contains("*.pdf;*.PDF;"));
    }

    /// Registra e remove numa pasta temporária no lugar da pasta do usuário.
    #[test]
    fn register_and_unregister_in_temp_home() {
        let root = std::env::temp_dir().join(format!("orbita-linux-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::remove_var("APPIMAGE");

        // Thunar já com uma ação do usuário: ela tem que continuar lá.
        let uca = root.join("config/Thunar/uca.xml");
        std::fs::create_dir_all(uca.parent().unwrap()).unwrap();
        let user_action = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n<action>\n\t<unique-id>1</unique-id>\n</action>\n</actions>\n";
        std::fs::write(&uca, user_action).unwrap();

        register(Path::new("/opt/Órbita/orbita"), EXTS).unwrap();
        let data = root.join("data");
        let open_with = std::fs::read_to_string(open_with_file(&data)).unwrap();
        assert!(open_with.contains("Exec=\"/opt/Órbita/orbita\" %F"));
        assert!(std::fs::read_to_string(nautilus_script(&data))
            .unwrap()
            .contains("exec '/opt/Órbita/orbita' \"$@\""));
        assert!(dolphin_files(&data).iter().all(|f| f.exists()));
        assert!(nemo_file(&data).exists());
        let thunar = std::fs::read_to_string(&uca).unwrap();
        assert!(thunar.contains(ID) && thunar.contains("<unique-id>1</unique-id>"));

        // Registrar de novo não duplica.
        register(Path::new("/opt/Órbita/orbita"), EXTS).unwrap();
        assert_eq!(
            std::fs::read_to_string(&uca).unwrap().matches(ID).count(),
            1
        );

        unregister(EXTS).unwrap();
        assert!(!open_with_file(&data).exists());
        assert!(!nautilus_script(&data).exists());
        assert!(!nemo_file(&data).exists());
        assert_eq!(std::fs::read_to_string(&uca).unwrap(), user_action);

        let _ = std::fs::remove_dir_all(&root);
    }
}
