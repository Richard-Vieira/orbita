use std::path::Path;

use serde::Serialize;

/// Formatos que o Órbita reconhece como entrada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Png,
    Jpg,
    Webp,
    Bmp,
}

impl Format {
    /// Identifica o formato pela extensão (sem diferenciar maiúsculas).
    pub fn from_path(path: &Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "pdf" => Format::Pdf,
            "png" => Format::Png,
            "jpg" | "jpeg" | "jfif" => Format::Jpg,
            "webp" => Format::Webp,
            "bmp" | "dib" => Format::Bmp,
            _ => return None,
        })
    }

    pub fn is_image(self) -> bool {
        !matches!(self, Format::Pdf)
    }

    /// Extensões tratadas (usadas pelo registro no menu do Explorer).
    pub const EXTENSIONS: &'static [&'static str] =
        &["pdf", "png", "jpg", "jpeg", "jfif", "webp", "bmp"];
}
