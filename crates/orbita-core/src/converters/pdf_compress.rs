//! Comprimir PDF.
//!
//! Estratégia (sem Ghostscript):
//! 1. Descobre em que tamanho cada imagem aparece na página (resolução efetiva).
//! 2. Imagens acima de 200 dpi são reduzidas para 150 dpi e regravadas em JPG.
//!    Imagens que já estão em resolução boa só são regravadas se o ganho for grande.
//! 3. Só mexe em imagens "seguras" (RGB/cinza de 8 bits, JPG ou Flate); máscaras,
//!    CMYK, JBIG2, etc. ficam intactas. Texto e vetores nunca são tocados.
//! 4. Regrava a estrutura com object streams (compactação sem perda).
//! 5. Confere o resultado com o pdfium (mesmo número de páginas, todas abrem) e só
//!    entrega se ficou pelo menos 10% menor. Caso contrário: "já está otimizado".

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, GrayImage, ImageFormat, RgbImage};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use pdfium_render::prelude::PdfRenderConfig;

use super::{imaging, single_input, Converter, Output};
use crate::error::{ConvertError, Result};
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{map_io, parent_dir, stem, unique_path, write_atomic};

/// Acima desta resolução efetiva, a imagem é reduzida...
const TRIGGER_DPI: f32 = 200.0;
/// ...para esta resolução (boa para tela e para impressão comum).
const TARGET_DPI: f32 = 150.0;
const JPEG_QUALITY: u8 = 75;
/// Ganho mínimo para valer a pena entregar um arquivo novo.
const MIN_GAIN: f64 = 0.10;
/// Profundidade máxima de Form XObjects aninhados.
const MAX_DEPTH: u32 = 12;

pub struct CompressPdf;

impl Converter for CompressPdf {
    fn id(&self) -> &'static str {
        "pdf.compress"
    }
    fn label(&self) -> &'static str {
        "Comprimir"
    }
    fn icon(&self) -> &'static str {
        "compress"
    }
    fn accepts(&self, format: Format) -> bool {
        format == Format::Pdf
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let input = single_input(inputs);
        let original_size = fs::metadata(input).map_err(|e| map_io(e, input))?.len();

        let mut doc = Document::load(input)
            .map_err(|e| ConvertError::Corrupt(format!("não foi possível ler o PDF ({e})")))?;
        if doc.was_encrypted() || doc.is_encrypted() {
            return Err(ConvertError::PasswordProtected);
        }
        let page_count = doc.get_pages().len();
        ctx.progress(0.05);

        let usage = image_usage(&doc);
        let masks = smask_targets(&doc);
        let candidates: Vec<(ObjectId, f32)> =
            usage.into_iter().filter(|(id, _)| !masks.contains(id)).collect();

        for (i, (id, dpi)) in candidates.iter().enumerate() {
            ctx.check_cancel()?;
            if let Some(replacement) = recompress_image(&doc, *id, *dpi) {
                doc.objects.insert(*id, Object::Stream(replacement.image));
                if let Some((mask_id, mask)) = replacement.smask {
                    doc.objects.insert(mask_id, Object::Stream(mask));
                }
            }
            ctx.progress(0.05 + 0.75 * (i + 1) as f32 / candidates.len() as f32);
        }
        doc.compress();

        let target = unique_path(&parent_dir(input), &format!("{} (comprimido)", stem(input)), "pdf");
        write_atomic(&target, |tmp| {
            let file = File::create(tmp).map_err(|e| map_io(e, tmp))?;
            doc.save_modern(&mut BufWriter::new(file))
                .map_err(|e| ConvertError::Other(format!("Falha ao gravar o PDF: {e}")))?;
            ctx.progress(0.85);

            let new_size = fs::metadata(tmp)?.len();
            if new_size as f64 > original_size as f64 * (1.0 - MIN_GAIN) {
                return Err(ConvertError::AlreadyOptimized);
            }
            validate(ctx, tmp, page_count)
        })?;
        ctx.progress(1.0);
        Ok(Output::single(target))
    }
}

/// Confere se o PDF gerado abre, tem o mesmo número de páginas e todas renderizam.
fn validate(ctx: &Ctx, path: &Path, expected_pages: usize) -> Result<()> {
    let failed = || ConvertError::Other("Não foi possível comprimir este PDF com segurança".into());
    let pdfium = ctx.pdfium.get()?;
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|_| failed())?;
    if doc.pages().len() as usize != expected_pages {
        return Err(failed());
    }
    let config = PdfRenderConfig::new().scale_page_by_factor(0.1);
    for page in doc.pages().iter() {
        page.render_with_config(&config).map_err(|_| failed())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Resolução efetiva de cada imagem
// ---------------------------------------------------------------------------

/// Matriz de transformação PDF [a b c d e f].
#[derive(Clone, Copy)]
struct Matrix([f32; 6]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn from_operands(ops: &[Object]) -> Option<Matrix> {
        if ops.len() != 6 {
            return None;
        }
        let mut m = [0.0; 6];
        for (slot, obj) in m.iter_mut().zip(ops) {
            *slot = number(obj)?;
        }
        Some(Matrix(m))
    }

    /// `self × other` (convenção de vetor-linha do PDF).
    fn then(self, other: Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [oa, ob, oc, od, oe, of] = other.0;
        Matrix([
            a * oa + b * oc,
            a * ob + b * od,
            c * oa + d * oc,
            c * ob + d * od,
            e * oa + f * oc + oe,
            e * ob + f * od + of,
        ])
    }
}

fn number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

fn resolve<'a>(doc: &'a Document, obj: &'a Object) -> &'a Object {
    doc.dereference(obj).map(|(_, o)| o).unwrap_or(obj)
}

fn name_is(dict: &Dictionary, key: &[u8], expected: &[u8]) -> bool {
    matches!(dict.get(key), Ok(Object::Name(n)) if n == expected)
}

fn int(dict: &Dictionary, key: &[u8]) -> Option<u32> {
    match dict.get(key) {
        Ok(Object::Integer(i)) if *i > 0 => Some(*i as u32),
        _ => None,
    }
}

/// Para cada imagem usada nas páginas: a MENOR resolução efetiva em que ela
/// aparece (se a mesma imagem aparece grande em algum lugar, preserva para lá).
fn image_usage(doc: &Document) -> HashMap<ObjectId, f32> {
    let mut usage = HashMap::new();
    for (_, page_id) in doc.get_pages() {
        let Ok(content) = Content::decode(&doc.get_page_content(page_id)) else { continue };
        let mut resources: Vec<&Dictionary> = Vec::new();
        if let Ok((direct, inherited)) = doc.get_page_resources(page_id) {
            resources.extend(direct);
            resources.extend(inherited.into_iter().filter_map(|id| doc.get_dictionary(id).ok()));
        }
        walk(doc, &content.operations, &resources, Matrix::IDENTITY, &mut usage, 0);
    }
    usage
}

fn find_xobject(doc: &Document, resources: &[&Dictionary], name: &[u8]) -> Option<ObjectId> {
    resources.iter().find_map(|res| {
        let xobjects = resolve(doc, res.get(b"XObject").ok()?).as_dict().ok()?;
        xobjects.get(name).ok()?.as_reference().ok()
    })
}

fn walk(
    doc: &Document,
    ops: &[Operation],
    resources: &[&Dictionary],
    mut ctm: Matrix,
    usage: &mut HashMap<ObjectId, f32>,
    depth: u32,
) {
    let mut stack = Vec::new();
    for op in ops {
        match op.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(ctm),
            "cm" => {
                if let Some(m) = Matrix::from_operands(&op.operands) {
                    ctm = m.then(ctm);
                }
            }
            "Do" => {
                let Some(Ok(name)) = op.operands.first().map(Object::as_name) else { continue };
                let Some(id) = find_xobject(doc, resources, name) else { continue };
                let Ok(stream) = doc.get_object(id).and_then(Object::as_stream) else { continue };
                let dict = &stream.dict;
                if name_is(dict, b"Subtype", b"Image") {
                    let (Some(w), Some(h)) = (int(dict, b"Width"), int(dict, b"Height")) else { continue };
                    let [a, b, c, d, ..] = ctm.0;
                    let (w_pt, h_pt) = (a.hypot(b), c.hypot(d));
                    if w_pt < 0.5 || h_pt < 0.5 {
                        continue;
                    }
                    let dpi = (w as f32 / (w_pt / 72.0)).min(h as f32 / (h_pt / 72.0));
                    usage.entry(id).and_modify(|v: &mut f32| *v = v.min(dpi)).or_insert(dpi);
                } else if name_is(dict, b"Subtype", b"Form") && depth < MAX_DEPTH {
                    let matrix = dict
                        .get(b"Matrix")
                        .ok()
                        .and_then(|m| m.as_array().ok())
                        .and_then(|m| Matrix::from_operands(m))
                        .unwrap_or(Matrix::IDENTITY);
                    let mut inner: Vec<&Dictionary> = Vec::new();
                    if let Some(own) = dict.get(b"Resources").ok().and_then(|r| resolve(doc, r).as_dict().ok()) {
                        inner.push(own);
                    }
                    inner.extend_from_slice(resources);
                    let Ok(bytes) = stream.decompressed_content() else { continue };
                    let Ok(content) = Content::decode(&bytes) else { continue };
                    walk(doc, &content.operations, &inner, matrix.then(ctm), usage, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Imagens usadas como máscara suave (SMask) de outra: tratadas junto com a dona.
fn smask_targets(doc: &Document) -> HashSet<ObjectId> {
    doc.objects
        .values()
        .filter_map(|o| o.as_stream().ok())
        .filter_map(|s| s.dict.get(b"SMask").ok()?.as_reference().ok())
        .collect()
}

// ---------------------------------------------------------------------------
// Recompressão de uma imagem
// ---------------------------------------------------------------------------

struct Replacement {
    image: Stream,
    smask: Option<(ObjectId, Stream)>,
}

#[derive(Clone, Copy, PartialEq)]
enum Codec {
    Dct,
    Flate,
}

/// Único filtro da imagem, se for um que sabemos tratar.
fn codec(dict: &Dictionary) -> Option<Codec> {
    let name = match dict.get(b"Filter").ok()? {
        Object::Name(n) => n.as_slice(),
        Object::Array(a) if a.len() == 1 => a[0].as_name().ok()?,
        _ => return None,
    };
    match name {
        b"DCTDecode" => Some(Codec::Dct),
        b"FlateDecode" => Some(Codec::Flate),
        _ => None,
    }
}

/// Número de componentes de cor (1 = cinza, 3 = RGB) para espaços de cor seguros.
fn components(doc: &Document, dict: &Dictionary) -> Option<u32> {
    match resolve(doc, dict.get(b"ColorSpace").ok()?) {
        Object::Name(n) if n == b"DeviceGray" => Some(1),
        Object::Name(n) if n == b"DeviceRGB" => Some(3),
        Object::Array(a) if a.len() == 2 && a[0].as_name().ok()? == b"ICCBased" => {
            let icc = resolve(doc, &a[1]).as_stream().ok()?;
            match icc.dict.get(b"N").ok()? {
                Object::Integer(n @ (1 | 3)) => Some(*n as u32),
                _ => None,
            }
        }
        _ => None,
    }
}

fn decode_pixels(stream: &Stream, codec: Codec, w: u32, h: u32, comps: u32) -> Option<DynamicImage> {
    let image = match codec {
        Codec::Dct => {
            let img = image::load_from_memory_with_format(&stream.content, ImageFormat::Jpeg).ok()?;
            if comps == 1 {
                DynamicImage::ImageLuma8(img.to_luma8())
            } else {
                DynamicImage::ImageRgb8(img.to_rgb8())
            }
        }
        Codec::Flate => {
            let raw = stream.decompressed_content().ok()?;
            if raw.len() != (w * h * comps) as usize {
                return None;
            }
            if comps == 1 {
                DynamicImage::ImageLuma8(GrayImage::from_raw(w, h, raw)?)
            } else {
                DynamicImage::ImageRgb8(RgbImage::from_raw(w, h, raw)?)
            }
        }
    };
    (image.width() == w && image.height() == h).then_some(image)
}

fn recompress_image(doc: &Document, id: ObjectId, dpi: f32) -> Option<Replacement> {
    let stream = doc.get_object(id).ok()?.as_stream().ok()?;
    let dict = &stream.dict;

    // Só imagens "comuns": nada de máscaras, arrays Decode ou profundidade != 8.
    let is_mask = matches!(dict.get(b"ImageMask"), Ok(Object::Boolean(true)));
    if is_mask || dict.has(b"Decode") || dict.has(b"Mask") || int(dict, b"BitsPerComponent") != Some(8) {
        return None;
    }
    let codec = codec(dict)?;
    if codec == Codec::Flate && dict.has(b"DecodeParms") {
        return None;
    }
    let comps = components(doc, dict)?;
    let (w, h) = (int(dict, b"Width")?, int(dict, b"Height")?);

    let scale = if dpi > TRIGGER_DPI { TARGET_DPI / dpi } else { 1.0 };
    let resized = scale < 1.0;
    if !resized && (w as u64 * h as u64) < 128 * 128 {
        return None;
    }

    // Se tiver máscara de transparência, ela precisa acompanhar a redução.
    let smask_id = dict.get(b"SMask").ok().and_then(|o| o.as_reference().ok());
    if resized && smask_id.is_some() && !smask_is_simple(doc, smask_id?) {
        return None;
    }

    let mut pixels = decode_pixels(stream, codec, w, h, comps)?;
    let (nw, nh) = if resized {
        let nw = ((w as f32 * scale).round() as u32).max(1);
        let nh = ((h as f32 * scale).round() as u32).max(1);
        pixels = pixels.resize_exact(nw, nh, FilterType::CatmullRom);
        (nw, nh)
    } else {
        (w, h)
    };
    let jpeg = imaging::encode_jpeg(&pixels, None, JPEG_QUALITY).ok()?;

    let old_len = stream.content.len();
    let worth_it = match (resized, codec) {
        (true, _) => jpeg.len() < old_len,
        (false, Codec::Dct) => jpeg.len() * 10 < old_len * 7,
        (false, Codec::Flate) => jpeg.len() * 2 < old_len,
    };
    if !worth_it {
        return None;
    }

    let mut new_dict = dict.clone();
    new_dict.set("Width", nw as i64);
    new_dict.set("Height", nh as i64);
    new_dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
    new_dict.remove(b"DecodeParms");
    let image = Stream::new(new_dict, jpeg).with_compression(false);

    let smask = match (resized, smask_id) {
        (true, Some(mask_id)) => Some((mask_id, resize_smask(doc, mask_id, nw, nh)?)),
        _ => None,
    };
    Some(Replacement { image, smask })
}

fn smask_is_simple(doc: &Document, id: ObjectId) -> bool {
    let Ok(stream) = doc.get_object(id).and_then(Object::as_stream) else { return false };
    let d = &stream.dict;
    !d.has(b"Matte")
        && !d.has(b"Decode")
        && int(d, b"BitsPerComponent") == Some(8)
        && name_is(d, b"ColorSpace", b"DeviceGray")
        && (!d.has(b"Filter") || (codec(d) == Some(Codec::Flate) && !d.has(b"DecodeParms")))
}

fn resize_smask(doc: &Document, id: ObjectId, nw: u32, nh: u32) -> Option<Stream> {
    let stream = doc.get_object(id).ok()?.as_stream().ok()?;
    let (w, h) = (int(&stream.dict, b"Width")?, int(&stream.dict, b"Height")?);
    let raw = stream.decompressed_content().ok()?;
    let mask = GrayImage::from_raw(w, h, raw)?;
    let resized = image::imageops::resize(&mask, nw, nh, FilterType::CatmullRom);

    let mut dict = stream.dict.clone();
    dict.set("Width", nw as i64);
    dict.set("Height", nh as i64);
    dict.remove(b"Filter");
    dict.remove(b"DecodeParms");
    let mut new = Stream::new(dict, resized.into_raw());
    new.compress().ok()?;
    Some(new)
}

#[cfg(test)]
mod tests {
    use super::Matrix;

    #[test]
    fn matrix_concatenation_scales() {
        let scale = Matrix([2.0, 0.0, 0.0, 3.0, 0.0, 0.0]);
        let translate = Matrix([1.0, 0.0, 0.0, 1.0, 10.0, 20.0]);
        let m = scale.then(translate);
        assert_eq!(m.0, [2.0, 0.0, 0.0, 3.0, 10.0, 20.0]);
    }
}
