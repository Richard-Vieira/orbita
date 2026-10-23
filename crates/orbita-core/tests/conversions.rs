//! Testes de ponta a ponta das conversões. Os arquivos de entrada são gerados
//! aqui mesmo (nada de binários no repositório).
//!
//! Precisam do pdfium: rode antes `scripts/fetch-pdfium.ps1` (ou `.sh`).

use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use orbita_core::{CancelToken, ConvertError, Engine};

fn engine() -> Engine {
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/pdfium");
    Engine::new(vec![vendor.join("bin"), vendor.join("lib")])
}

fn run(engine: &Engine, action: &str, inputs: &[PathBuf]) -> Result<Vec<PathBuf>, ConvertError> {
    engine.run(action, inputs, &|_| {}, &CancelToken::default()).map(|o| o.created)
}

/// PNG com transparência (um círculo azul sobre fundo transparente).
fn make_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let img = RgbaImage::from_fn(w, h, |x, y| {
        let (dx, dy) = (x as f32 - w as f32 / 2.0, y as f32 - h as f32 / 2.0);
        if dx.hypot(dy) < w.min(h) as f32 / 3.0 {
            Rgba([60, 110, 220, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    });
    let path = dir.join(name);
    img.save(&path).unwrap();
    path
}

/// PDF mínimo com texto em duas páginas (escrito à mão, fonte Helvetica).
fn make_text_pdf(dir: &Path, name: &str) -> PathBuf {
    let pages = ["Ola Orbita, pagina um.", "Segunda pagina do teste."];
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        format!("<< /Type /Pages /Kids [{}] /Count {} >>", (0..pages.len()).map(|i| format!("{} 0 R", 4 + i * 2)).collect::<Vec<_>>().join(" "), pages.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
    ];
    for (i, text) in pages.iter().enumerate() {
        let content = format!("BT /F1 18 Tf 72 720 Td ({text}) Tj ET");
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            5 + i * 2
        ));
        objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
    }
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{obj}\nendobj\n", i + 1));
    }
    let xref = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for off in offsets {
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
    let path = dir.join(name);
    fs::write(&path, pdf).unwrap();
    path
}

#[test]
fn image_conversions() {
    let dir = tempfile::tempdir().unwrap();
    let png = make_png(dir.path(), "logo.png", 300, 200);
    let engine = engine();

    let jpg = run(&engine, "img.jpg", &[png.clone()]).unwrap();
    assert_eq!(jpg[0].file_name().unwrap(), "logo.jpg");
    // Transparência vira branco no JPG.
    let decoded = image::open(&jpg[0]).unwrap().to_rgb8();
    assert!(decoded.get_pixel(2, 2).0.iter().all(|&c| c > 245));

    let webp = run(&engine, "img.webp", &[png.clone()]).unwrap();
    assert_eq!(image::open(&webp[0]).unwrap().width(), 300);

    let ico = run(&engine, "img.ico", &[png.clone()]).unwrap();
    assert!(fs::metadata(&ico[0]).unwrap().len() > 0);

    // Nunca sobrescreve: a segunda conversão ganha o sufixo (1).
    let again = run(&engine, "img.jpg", &[png]).unwrap();
    assert_eq!(again[0].file_name().unwrap(), "logo (1).jpg");
}

#[test]
fn pdf_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    let png = make_png(dir.path(), "foto.png", 400, 300);

    let pdf = run(&engine, "img.pdf", &[png]).unwrap().remove(0);
    let back = run(&engine, "pdf.png", &[pdf]).unwrap().remove(0);
    let page = image::open(back).unwrap();
    // 400 px a 96 dpi = 300 pt; renderizado a 200 dpi = ~833 px.
    assert!((page.width() as i32 - 833).abs() <= 2, "largura {}", page.width());
}

#[test]
fn pdf_text_and_pages() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    let pdf = make_text_pdf(dir.path(), "texto.pdf");

    let txt = run(&engine, "pdf.txt", &[pdf.clone()]).unwrap();
    let text = fs::read_to_string(&txt[0]).unwrap();
    assert!(text.contains("Ola Orbita, pagina um."));
    assert!(text.contains("Segunda pagina"));

    // Duas páginas -> pasta "texto (JPG)" com uma imagem por página.
    let folder = run(&engine, "pdf.jpg", &[pdf]).unwrap().remove(0);
    assert_eq!(folder.file_name().unwrap(), "texto (JPG)");
    assert_eq!(fs::read_dir(folder).unwrap().count(), 2);
}

#[test]
fn merge_pdfs_and_images() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    let a = make_text_pdf(dir.path(), "a.pdf");
    let b = make_png(dir.path(), "b.png", 200, 200);

    let merged = run(&engine, "merge", &[b, a]).unwrap().remove(0);
    assert_eq!(merged.file_name().unwrap(), "Juntados - a.pdf");
    let doc = lopdf::Document::load(&merged).unwrap();
    assert_eq!(doc.get_pages().len(), 3);
}

#[test]
fn compress_then_report_already_optimized() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    let pdf = make_text_pdf(dir.path(), "pequeno.pdf");

    // Primeira vez: os fluxos sem compactação ficam menores.
    let compressed = run(&engine, "pdf.compress", &[pdf]).unwrap().remove(0);
    assert_eq!(compressed.file_name().unwrap(), "pequeno (comprimido).pdf");

    // De novo no resultado: nada mais a ganhar.
    match run(&engine, "pdf.compress", &[compressed]) {
        Err(ConvertError::AlreadyOptimized) => {}
        other => panic!("esperava 'já otimizado', veio {other:?}"),
    }
    // Nenhum arquivo extra (nem temporário) foi deixado para trás.
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn compress_downsamples_large_photos() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    // "Foto" de 2480x3508 (A4 a 300 dpi) com ruído, para não compactar fácil.
    let mut seed = 7u32;
    let photo = image::RgbImage::from_fn(2480, 3508, |x, y| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let n = (seed >> 24) as u8 / 4;
        image::Rgb([(x / 10) as u8 ^ n, (y / 14) as u8 ^ n, 128u8.wrapping_add(n)])
    });
    let jpg = dir.path().join("foto.jpg");
    photo.save(&jpg).unwrap();

    let pdf = run(&engine, "img.pdf", &[jpg]).unwrap().remove(0);
    let compressed = run(&engine, "pdf.compress", &[pdf.clone()]).unwrap().remove(0);
    let (before, after) = (fs::metadata(&pdf).unwrap().len(), fs::metadata(&compressed).unwrap().len());
    assert!(after * 2 < before, "esperava reduzir pela metade: {before} -> {after}");
}

#[test]
fn scanned_pdf_has_no_text() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine();
    let png = make_png(dir.path(), "scan.png", 300, 300);
    let pdf = run(&engine, "img.pdf", &[png]).unwrap().remove(0);
    assert!(matches!(run(&engine, "pdf.txt", &[pdf]), Err(ConvertError::NoText)));
}

#[test]
fn actions_depend_on_selection() {
    let ids = |files: &[&str]| -> Vec<&'static str> {
        let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
        orbita_core::actions_for(&paths).into_iter().map(|a| a.id).collect()
    };
    assert_eq!(ids(&["a.pdf"]), ["pdf.png", "pdf.jpg", "pdf.txt", "pdf.compress"]);
    assert_eq!(ids(&["a.jpg"]), ["img.png", "img.webp", "img.ico", "img.pdf"]);
    assert!(ids(&["a.pdf", "b.pdf"]).contains(&"merge"));
    assert_eq!(ids(&["a.pdf", "b.png"]), ["merge"]);
    assert!(ids(&["a.docx"]).is_empty());
}
