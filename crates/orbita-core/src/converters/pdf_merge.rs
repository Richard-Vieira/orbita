//! Vários PDFs e/ou imagens -> um PDF só, em ordem alfabética.

use std::path::PathBuf;

use super::{image_to_pdf::add_image_page, Converter, Mode, Output};
use crate::error::{ConvertError, Result};
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{natural_cmp, parent_dir, stem, unique_path, write_atomic};

pub struct Merge;

impl Converter for Merge {
    fn id(&self) -> &'static str {
        "merge"
    }
    fn label(&self) -> &'static str {
        "Juntar"
    }
    fn icon(&self) -> &'static str {
        "merge"
    }
    fn mode(&self) -> Mode {
        Mode::Combine
    }
    fn accepts(&self, _format: Format) -> bool {
        true
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let mut inputs = inputs.to_vec();
        inputs.sort_by(|a, b| {
            natural_cmp(&a.file_name().unwrap_or_default().to_string_lossy(), &b.file_name().unwrap_or_default().to_string_lossy())
        });

        let pdfium = ctx.pdfium.get()?;
        let mut out = pdfium.create_new_pdf()?;
        let (mut lost_forms, mut lost_bookmarks) = (false, false);

        for (i, path) in inputs.iter().enumerate() {
            ctx.check_cancel()?;
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let named = |e: ConvertError| match e {
                ConvertError::PasswordProtected => ConvertError::Other(format!("“{name}” está protegido por senha")),
                ConvertError::Corrupt(_) => ConvertError::Corrupt(format!("“{name}” está danificado")),
                other => other,
            };
            if Format::from_path(path) == Some(Format::Pdf) {
                let src = pdfium.load_pdf_from_file(path, None).map_err(|e| named(e.into()))?;
                lost_forms |= src.form().is_some();
                lost_bookmarks |= src.bookmarks().root().is_some();
                out.pages_mut().append(&src).map_err(|e| named(e.into()))?;
            } else {
                add_image_page(&mut out, path).map_err(named)?;
            }
            ctx.progress((i + 1) as f32 / inputs.len() as f32 * 0.9);
        }

        let first = &inputs[0];
        let target = unique_path(&parent_dir(first), &format!("Juntados - {}", stem(first)), "pdf");
        write_atomic(&target, |tmp| Ok(out.save_to_file(tmp)?))?;
        ctx.progress(1.0);

        let mut output = Output::single(target);
        if lost_forms {
            output.notes.push("Campos de formulário não ficam editáveis no PDF juntado".into());
        }
        if lost_bookmarks {
            output.notes.push("Marcadores (índice) não foram mantidos".into());
        }
        Ok(output)
    }
}
