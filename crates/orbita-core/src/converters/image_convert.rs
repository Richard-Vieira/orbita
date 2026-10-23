//! Imagem -> PNG, JPG, WEBP ou ICO.

use std::fs;
use std::path::PathBuf;

use super::{imaging, single_input, Converter, Output};
use crate::error::Result;
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{map_io, parent_dir, stem, unique_path, write_atomic};

#[derive(Debug, Clone, Copy)]
pub enum ImageTarget {
    Png,
    Jpg,
    Webp,
    Ico,
}

pub struct ToImage(pub ImageTarget);

impl Converter for ToImage {
    fn id(&self) -> &'static str {
        match self.0 {
            ImageTarget::Png => "img.png",
            ImageTarget::Jpg => "img.jpg",
            ImageTarget::Webp => "img.webp",
            ImageTarget::Ico => "img.ico",
        }
    }

    fn label(&self) -> &'static str {
        match self.0 {
            ImageTarget::Png => "PNG",
            ImageTarget::Jpg => "JPG",
            ImageTarget::Webp => "WEBP",
            ImageTarget::Ico => "Ícone",
        }
    }

    fn icon(&self) -> &'static str {
        match self.0 {
            ImageTarget::Png => "png",
            ImageTarget::Jpg => "jpg",
            ImageTarget::Webp => "webp",
            ImageTarget::Ico => "ico",
        }
    }

    fn accepts(&self, format: Format) -> bool {
        format.is_image()
    }

    fn output_format(&self) -> Option<Format> {
        match self.0 {
            ImageTarget::Png => Some(Format::Png),
            ImageTarget::Jpg => Some(Format::Jpg),
            ImageTarget::Webp => Some(Format::Webp),
            ImageTarget::Ico => None,
        }
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let input = single_input(inputs);
        let loaded = imaging::load(input)?;
        ctx.progress(0.4);
        ctx.check_cancel()?;

        let icc = loaded.icc.as_deref();
        let source_lossless = matches!(Format::from_path(input), Some(Format::Png | Format::Bmp));
        let (ext, bytes): (&str, Option<Vec<u8>>) = match self.0 {
            ImageTarget::Png => ("png", None),
            ImageTarget::Jpg => ("jpg", Some(imaging::encode_jpeg(&loaded.image, icc, 90)?)),
            ImageTarget::Webp => ("webp", Some(imaging::encode_webp(&loaded.image, source_lossless)?)),
            ImageTarget::Ico => ("ico", Some(imaging::encode_ico(&loaded.image)?)),
        };
        ctx.progress(0.8);

        let target = unique_path(&parent_dir(input), &stem(input), ext);
        write_atomic(&target, |tmp| match bytes {
            Some(bytes) => fs::write(tmp, bytes).map_err(|e| map_io(e, tmp)),
            None => imaging::write_png(&loaded.image, icc, tmp),
        })?;
        ctx.progress(1.0);
        Ok(Output::single(target))
    }
}
