//! PDF -> PNG ou JPG, uma imagem por página.
//!
//! PDF de uma página vira `arquivo.png` ao lado do original. Com várias páginas,
//! as imagens vão para uma pasta `arquivo (PNG)` para não espalhar dezenas de
//! arquivos na pasta do usuário.

use std::fs;
use std::path::PathBuf;

use pdfium_render::prelude::*;

use super::{imaging, single_input, Converter, Output};
use crate::error::{ConvertError, Result};
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{map_io, parent_dir, stem, unique_path, write_atomic};

/// Resolução padrão: texto pequeno continua nítido.
const DPI: f32 = 200.0;
/// Limite de pixels por página (pôsteres e plantas gigantes são reduzidos).
const MAX_PIXELS: f32 = 40_000_000.0;

#[derive(Debug, Clone, Copy)]
pub enum Raster {
    Png,
    Jpg,
}

impl Raster {
    fn ext(self) -> &'static str {
        match self {
            Raster::Png => "png",
            Raster::Jpg => "jpg",
        }
    }
}

pub struct PdfToImage(pub Raster);

impl Converter for PdfToImage {
    fn id(&self) -> &'static str {
        match self.0 {
            Raster::Png => "pdf.png",
            Raster::Jpg => "pdf.jpg",
        }
    }
    fn label(&self) -> &'static str {
        match self.0 {
            Raster::Png => "PNG",
            Raster::Jpg => "JPG",
        }
    }
    fn icon(&self) -> &'static str {
        self.0.ext()
    }
    fn accepts(&self, format: Format) -> bool {
        format == Format::Pdf
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let input = single_input(inputs);
        let ext = self.0.ext();
        let pdfium = ctx.pdfium.get()?;
        let doc = pdfium.load_pdf_from_file(input, None)?;
        let pages = doc.pages();
        let count = pages.len() as usize;
        if count == 0 {
            return Err(ConvertError::Corrupt("o PDF não tem páginas".into()));
        }

        let parent = parent_dir(input);
        let base = stem(input);
        let folder = if count > 1 {
            let dir = unique_path(&parent, &format!("{base} ({})", ext.to_uppercase()), "");
            fs::create_dir(&dir).map_err(|e| map_io(e, &dir))?;
            Some(dir)
        } else {
            None
        };

        let result = (|| -> Result<PathBuf> {
            let digits = count.to_string().len().max(2);
            let mut single_target = None;
            for (i, page) in pages.iter().enumerate() {
                ctx.check_cancel()?;
                let image = render_page(&page)?;
                let target = match &folder {
                    Some(dir) => dir.join(format!("página {:0digits$}.{ext}", i + 1)),
                    None => unique_path(&parent, &base, ext),
                };
                write_atomic(&target, |tmp| match self.0 {
                    Raster::Png => imaging::write_png(&image, None, tmp),
                    Raster::Jpg => {
                        let bytes = imaging::encode_jpeg(&image, None, 90)?;
                        fs::write(tmp, bytes).map_err(|e| map_io(e, tmp))
                    }
                })?;
                single_target = Some(target);
                ctx.progress((i + 1) as f32 / count as f32);
            }
            Ok(folder.clone().or(single_target).expect("ao menos uma página"))
        })();

        match result {
            Ok(created) => Ok(Output::single(created)),
            Err(e) => {
                // Não deixa uma pasta pela metade para trás.
                if let Some(dir) = &folder {
                    let _ = fs::remove_dir_all(dir);
                }
                Err(e)
            }
        }
    }
}

fn render_page(page: &PdfPage) -> Result<image::DynamicImage> {
    let mut scale = DPI / 72.0;
    let (w, h) = (page.width().value * scale, page.height().value * scale);
    if w * h > MAX_PIXELS {
        scale *= (MAX_PIXELS / (w * h)).sqrt();
    }
    let config = PdfRenderConfig::new()
        .scale_page_by_factor(scale)
        .set_clear_color(PdfColor::WHITE)
        .render_form_data(true);
    let bitmap = page.render_with_config(&config)?;
    Ok(image::DynamicImage::ImageRgb8(bitmap.as_image()?.into_rgb8()))
}
