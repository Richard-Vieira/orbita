//! Informações de arquivo enviadas para a interface.

use std::path::{Path, PathBuf};

use orbita_core::Format;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub kind: &'static str,
    pub is_dir: bool,
}

impl FileInfo {
    pub fn from_path(path: &Path) -> FileInfo {
        let meta = std::fs::metadata(path).ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let kind = match Format::from_path(path) {
            _ if is_dir => "folder",
            Some(Format::Pdf) => "pdf",
            Some(_) => "image",
            None if ext == "txt" => "text",
            None if ext == "ico" => "icon",
            None => "file",
        };
        FileInfo {
            path: path.display().to_string(),
            name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            size: meta.filter(|m| m.is_file()).map_or(0, |m| m.len()),
            kind,
            is_dir,
        }
    }
}

/// Arquivos (não pastas) que existem, a partir de argumentos de linha de comando.
pub fn existing_files<I, S>(args: I) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .map(|a| PathBuf::from(a.as_ref()))
        .filter(|p| p.is_file())
        .collect()
}
