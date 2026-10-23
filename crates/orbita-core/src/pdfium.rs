//! Carregamento preguiçoso da biblioteca pdfium (pdfium.dll / libpdfium.so).

use std::path::PathBuf;
use std::sync::OnceLock;

use pdfium_render::prelude::Pdfium;

use crate::error::{ConvertError, Result};

/// O pdfium só pode ser carregado uma vez por processo; todas as instâncias de
/// [`PdfiumHost`] compartilham o mesmo carregamento.
static INSTANCE: OnceLock<std::result::Result<Pdfium, String>> = OnceLock::new();

/// Localiza e carrega o pdfium uma única vez, na primeira conversão de PDF.
pub struct PdfiumHost {
    search_dirs: Vec<PathBuf>,
}

impl PdfiumHost {
    /// `search_dirs`: pastas onde procurar a biblioteca, em ordem de preferência.
    /// A variável de ambiente `ORBITA_PDFIUM_DIR` sempre tem prioridade.
    pub fn new(search_dirs: Vec<PathBuf>) -> Self {
        let mut dirs = Vec::new();
        if let Some(dir) = std::env::var_os("ORBITA_PDFIUM_DIR") {
            dirs.push(PathBuf::from(dir));
        }
        dirs.extend(search_dirs);
        Self { search_dirs: dirs }
    }

    pub fn get(&self) -> Result<&Pdfium> {
        INSTANCE
            .get_or_init(|| self.load())
            .as_ref()
            .map_err(|e| {
                log::error!("pdfium indisponível: {e}");
                ConvertError::PdfiumMissing
            })
    }

    fn load(&self) -> std::result::Result<Pdfium, String> {
        let mut last_error = String::from("nenhuma pasta de busca");
        for dir in &self.search_dirs {
            let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
            if !lib.exists() {
                continue;
            }
            match Pdfium::bind_to_library(&lib) {
                Ok(bindings) => {
                    log::info!("pdfium carregado de {}", lib.display());
                    return Ok(Pdfium::new(bindings));
                }
                Err(e) => last_error = format!("{}: {e:?}", lib.display()),
            }
        }
        Err(last_error)
    }
}
