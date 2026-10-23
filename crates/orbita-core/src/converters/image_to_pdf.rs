//! Imagem -> PDF (uma página do tamanho da imagem).
//!
//! JPGs entram no PDF sem recompressão (os bytes originais são copiados), então
//! não há perda de qualidade. Outros formatos entram sem perda (Flate), mantendo
//! a transparência.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

use super::{imaging, single_input, Converter, Output};
use crate::error::Result;
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{parent_dir, stem, unique_path, write_atomic};

/// Lado maior de uma folha A4, em pontos.
const A4_LONG_SIDE_PT: f32 = 842.0;

pub struct ImageToPdf;

impl Converter for ImageToPdf {
    fn id(&self) -> &'static str {
        "img.pdf"
    }
    fn label(&self) -> &'static str {
        "PDF"
    }
    fn icon(&self) -> &'static str {
        "pdf"
    }
    fn accepts(&self, format: Format) -> bool {
        format.is_image()
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let input = single_input(inputs);
        let pdfium = ctx.pdfium.get()?;
        let mut doc = pdfium.create_new_pdf()?;
        add_image_page(&mut doc, input)?;
        ctx.progress(0.7);
        ctx.check_cancel()?;

        let target = unique_path(&parent_dir(input), &stem(input), "pdf");
        write_atomic(&target, |tmp| Ok(doc.save_to_file(tmp)?))?;
        ctx.progress(1.0);
        Ok(Output::single(target))
    }
}

/// Tamanho da página em pontos: tamanho natural a 96 dpi, reduzido para caber
/// num A4 quando a imagem for maior que isso (fotos de celular, por exemplo).
fn page_size_pt(width_px: u32, height_px: u32) -> (f32, f32) {
    let (w, h) = (width_px as f32 * 0.75, height_px as f32 * 0.75);
    let scale = (A4_LONG_SIDE_PT / w.max(h)).min(1.0);
    (w * scale, h * scale)
}

/// Acrescenta ao documento uma página contendo a imagem.
pub fn add_image_page(doc: &mut PdfDocument, path: &Path) -> Result<()> {
    let loaded = imaging::load(path)?;
    let (w_pt, h_pt) = page_size_pt(loaded.image.width(), loaded.image.height());

    let mut object = if Format::from_path(path) == Some(Format::Jpg) {
        if loaded.reoriented {
            // A orientação EXIF não vale dentro do PDF: grava a foto já girada.
            let jpeg = imaging::encode_jpeg(&loaded.image, loaded.icc.as_deref(), 92)?;
            PdfPageImageObject::new_from_jpeg_reader(doc, Cursor::new(jpeg))?
        } else {
            PdfPageImageObject::new_from_jpeg_file(doc, path)?
        }
    } else {
        PdfPageImageObject::new(doc, &loaded.image)?
    };
    object.scale(w_pt, h_pt)?;

    let mut page = doc
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::Custom(PdfPoints::new(w_pt), PdfPoints::new(h_pt)))?;
    page.objects_mut().add_image_object(object)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::page_size_pt;

    #[test]
    fn small_images_keep_natural_size() {
        assert_eq!(page_size_pt(400, 200), (300.0, 150.0));
    }

    #[test]
    fn large_photos_fit_a4() {
        let (w, h) = page_size_pt(4000, 3000);
        assert!((w - 842.0).abs() < 0.01);
        assert!((h - 631.5).abs() < 0.01);
    }
}
