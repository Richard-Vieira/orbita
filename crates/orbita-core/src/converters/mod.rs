//! Cada conversão é um módulo que implementa [`Converter`].
//!
//! Para adicionar um formato novo:
//! 1. crie um arquivo neste diretório implementando [`Converter`];
//! 2. registre uma instância em [`all`].

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::format::Format;
use crate::job::Ctx;

pub mod image_convert;
pub mod image_to_pdf;
pub mod imaging;
pub mod pdf_compress;
pub mod pdf_merge;
pub mod pdf_render;
pub mod pdf_text;

/// Como o conversor trata uma seleção com vários arquivos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Converte cada arquivo separadamente.
    PerFile,
    /// Junta todos os arquivos selecionados num resultado só (exige 2 ou mais).
    Combine,
}

/// Resultado de uma conversão.
#[derive(Debug, Default)]
pub struct Output {
    /// Arquivos (ou pastas) criados.
    pub created: Vec<PathBuf>,
    /// Avisos curtos para o usuário (ex.: "formulários não preservados").
    pub notes: Vec<String>,
}

impl Output {
    pub fn single(path: PathBuf) -> Self {
        Self { created: vec![path], notes: Vec::new() }
    }
}

pub trait Converter: Send + Sync {
    /// Identificador estável (usado pela interface).
    fn id(&self) -> &'static str;
    /// Texto curto mostrado na fatia do menu.
    fn label(&self) -> &'static str;
    /// Nome do ícone de linha usado pela interface.
    fn icon(&self) -> &'static str;
    fn mode(&self) -> Mode {
        Mode::PerFile
    }
    /// Se este conversor sabe tratar arquivos desse formato.
    fn accepts(&self, format: Format) -> bool;
    /// Formato de saída, quando for um formato de entrada também. Serve para não
    /// oferecer "PNG" para um arquivo que já é PNG.
    fn output_format(&self) -> Option<Format> {
        None
    }
    /// Executa a conversão. Em `Mode::PerFile` recebe exatamente um arquivo.
    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output>;
}

/// Todos os conversores, na ordem em que aparecem no menu.
pub fn all() -> Vec<Box<dyn Converter>> {
    use image_convert::{ImageTarget, ToImage};
    use pdf_render::{PdfToImage, Raster};
    vec![
        Box::new(PdfToImage(Raster::Png)),
        Box::new(PdfToImage(Raster::Jpg)),
        Box::new(pdf_text::PdfToText),
        Box::new(pdf_compress::CompressPdf),
        Box::new(ToImage(ImageTarget::Png)),
        Box::new(ToImage(ImageTarget::Jpg)),
        Box::new(ToImage(ImageTarget::Webp)),
        Box::new(ToImage(ImageTarget::Ico)),
        Box::new(image_to_pdf::ImageToPdf),
        Box::new(pdf_merge::Merge),
    ]
}

/// Atalho usado pelos conversores de arquivo único.
pub(crate) fn single_input(inputs: &[PathBuf]) -> &Path {
    inputs.first().map(PathBuf::as_path).expect("conversor PerFile recebe um arquivo")
}
