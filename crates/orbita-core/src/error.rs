use std::path::PathBuf;

/// Erros de conversão. A mensagem (`Display`) é curta e pensada para o usuário final.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("Formato não suportado para esta conversão")]
    Unsupported,

    #[error("PDF protegido por senha")]
    PasswordProtected,

    #[error("Este PDF não tem texto (parece digitalizado)")]
    NoText,

    #[error("Este PDF já está otimizado")]
    AlreadyOptimized,

    #[error("Imagem grande demais para {format} (máx. {max} px por lado)")]
    TooLarge { format: &'static str, max: u32 },

    #[error("Não foi possível ler o arquivo: {0}")]
    Corrupt(String),

    #[error("Arquivo em uso ou sem permissão: {}", .0.display())]
    Access(PathBuf),

    #[error("O componente de PDF (pdfium) não foi encontrado")]
    PdfiumMissing,

    #[error("Conversão cancelada")]
    Cancelled,

    #[error("Falha ao gravar o arquivo: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl From<pdfium_render::prelude::PdfiumError> for ConvertError {
    fn from(e: pdfium_render::prelude::PdfiumError) -> Self {
        use pdfium_render::prelude::{PdfiumError, PdfiumInternalError};
        match e {
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
                ConvertError::PasswordProtected
            }
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError) => {
                ConvertError::Corrupt("PDF inválido ou danificado".into())
            }
            PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FileError) => {
                ConvertError::Corrupt("não foi possível abrir o PDF".into())
            }
            PdfiumError::IoError(io) => ConvertError::Io(io),
            other => ConvertError::Other(format!("Erro no PDF: {other:?}")),
        }
    }
}

impl From<image::ImageError> for ConvertError {
    fn from(e: image::ImageError) -> Self {
        match e {
            image::ImageError::IoError(io) => ConvertError::Io(io),
            image::ImageError::Unsupported(_) => ConvertError::Unsupported,
            other => ConvertError::Corrupt(other.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, ConvertError>;
