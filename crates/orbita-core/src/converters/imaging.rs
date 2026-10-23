//! Leitura e gravação de imagens compartilhadas pelos conversores.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::imageops::{self, FilterType};
use image::metadata::Orientation;
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader, RgbImage, RgbaImage};

use crate::error::{ConvertError, Result};
use crate::naming::map_io;

/// Imagem decodificada, já na orientação correta, com o perfil de cor original.
pub struct Loaded {
    pub image: DynamicImage,
    pub icc: Option<Vec<u8>>,
    /// `true` se a foto precisou ser girada/espelhada pela orientação EXIF.
    pub reoriented: bool,
}

pub fn load(path: &Path) -> Result<Loaded> {
    let reader = ImageReader::open(path)
        .map_err(|e| map_io(e, path))?
        .with_guessed_format()
        .map_err(|e| map_io(e, path))?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let icc = decoder.icc_profile().ok().flatten();
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(Loaded { image, icc, reoriented: orientation != Orientation::NoTransforms })
}

/// Compõe a imagem sobre fundo branco (JPG não tem transparência).
pub fn flatten_on_white(image: &DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = p[3] as u32;
        let mix = |c: u8| ((c as u32 * a + 255 * (255 - a) + 127) / 255) as u8;
        image::Rgb([mix(p[0]), mix(p[1]), mix(p[2])])
    })
}

pub fn write_png(image: &DynamicImage, icc: Option<&[u8]>, path: &Path) -> Result<()> {
    let file = File::create(path).map_err(|e| map_io(e, path))?;
    let mut encoder = PngEncoder::new_with_quality(
        BufWriter::new(file),
        CompressionType::Default,
        PngFilter::Adaptive,
    );
    if let Some(icc) = icc {
        let _ = encoder.set_icc_profile(icc.to_vec());
    }
    image.write_with_encoder(encoder)?;
    Ok(())
}

/// JPEG com subamostragem 4:2:0 (arquivos menores que o codificador da crate
/// `image` na mesma qualidade).
///
/// Não usamos tabelas Huffman otimizadas: combinadas com 4:2:0, o jpeg-encoder
/// gera arquivos que alguns decodificadores (ex.: zune-jpeg) leem com as cores
/// erradas. O ganho seria de só ~5%.
pub fn encode_jpeg(image: &DynamicImage, icc: Option<&[u8]>, quality: u8) -> Result<Vec<u8>> {
    use jpeg_encoder::{ColorType, Encoder, SamplingFactor};

    let (w, h) = (image.width(), image.height());
    if w > u16::MAX as u32 || h > u16::MAX as u32 {
        return Err(ConvertError::TooLarge { format: "JPG", max: u16::MAX as u32 });
    }
    let gray = matches!(image, DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_));
    let (pixels, color) = if gray {
        (image.to_luma8().into_raw(), ColorType::Luma)
    } else {
        (flatten_on_white(image).into_raw(), ColorType::Rgb)
    };

    let mut out = Vec::new();
    let mut encoder = Encoder::new(&mut out, quality);
    encoder.set_sampling_factor(SamplingFactor::R_4_2_0);
    if let Some(icc) = icc {
        let _ = encoder.add_icc_profile(icc);
    }
    encoder
        .encode(&pixels, w as u16, h as u16, color)
        .map_err(|e| ConvertError::Other(format!("Falha ao gerar JPG: {e}")))?;
    Ok(out)
}

const WEBP_MAX: u32 = 16383;

/// WebP com perda (qualidade 85). Para imagens "sem perda" de origem (PNG/BMP)
/// pequenas, tenta também a versão sem perda e fica com ela se não for muito maior.
pub fn encode_webp(image: &DynamicImage, prefer_lossless: bool) -> Result<Vec<u8>> {
    let (w, h) = (image.width(), image.height());
    if w > WEBP_MAX || h > WEBP_MAX {
        return Err(ConvertError::TooLarge { format: "WEBP", max: WEBP_MAX });
    }
    let rgba: RgbaImage = image.to_rgba8();
    let lossy = webp_encode(&rgba, Some(85.0))?;
    if prefer_lossless && (w as u64 * h as u64) <= 4_000_000 {
        let lossless = webp_encode(&rgba, None)?;
        if lossless.len() <= lossy.len() * 3 / 2 {
            return Ok(lossless);
        }
    }
    Ok(lossy)
}

/// `quality = None` gera WebP sem perda.
fn webp_encode(rgba: &RgbaImage, quality: Option<f32>) -> Result<Vec<u8>> {
    use libwebp_sys::{WebPEncodeLosslessRGBA, WebPEncodeRGBA, WebPFree};

    let (w, h) = (rgba.width() as i32, rgba.height() as i32);
    let stride = w * 4;
    let mut ptr: *mut u8 = std::ptr::null_mut();
    // SAFETY: o buffer tem w*h*4 bytes; a libwebp aloca `ptr`, que liberamos com WebPFree.
    let len = unsafe {
        match quality {
            Some(q) => WebPEncodeRGBA(rgba.as_ptr(), w, h, stride, q, &mut ptr),
            None => WebPEncodeLosslessRGBA(rgba.as_ptr(), w, h, stride, &mut ptr),
        }
    };
    if len == 0 || ptr.is_null() {
        return Err(ConvertError::Other("Falha ao gerar WEBP".into()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };
    unsafe { WebPFree(ptr.cast()) };
    Ok(bytes)
}

/// Ícone .ico com vários tamanhos (16 a 256 px). A imagem é centralizada num
/// quadrado transparente para não distorcer.
pub fn encode_ico(image: &DynamicImage) -> Result<Vec<u8>> {
    const SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

    let rgba = image.to_rgba8();
    let side = rgba.width().max(rgba.height());
    let mut square = RgbaImage::new(side, side);
    let (dx, dy) = ((side - rgba.width()) / 2, (side - rgba.height()) / 2);
    imageops::overlay(&mut square, &rgba, dx as i64, dy as i64);

    let mut frames = Vec::new();
    for size in SIZES.into_iter().filter(|&s| s <= side.max(48)) {
        let scaled = imageops::resize(&square, size, size, FilterType::Lanczos3);
        frames.push(IcoFrame::as_png(scaled.as_raw(), size, size, ExtendedColorType::Rgba8)?);
    }
    let mut out = Vec::new();
    IcoEncoder::new(&mut out).encode_images(&frames)?;
    Ok(out)
}
