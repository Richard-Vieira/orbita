//! Fundo do "vidro fosco": captura o pedaço da tela que ficará atrás da janela,
//! reduz e desfoca. O resultado vai para a interface como imagem PNG em base64
//! e nunca é gravado em disco.

use base64::Engine as _;

/// Quanto a captura é reduzida antes do desfoque (também deixa tudo leve).
const DOWNSCALE: u32 = 6;
const BLUR_RADIUS: i32 = 2;
const BLUR_PASSES: usize = 3;

/// Retângulo em pixels físicos da tela.
pub fn capture(x: i32, y: i32, width: u32, height: u32) -> Option<String> {
    if !orbita_platform::transparency_enabled() {
        return None;
    }
    let shot = orbita_platform::capture_screen(x, y, width, height)?;
    let (w, h) = ((shot.width / DOWNSCALE).max(1), (shot.height / DOWNSCALE).max(1));
    let mut small = downscale(&shot.rgba, shot.width, shot.height, w, h);
    for _ in 0..BLUR_PASSES {
        box_blur(&mut small, w, h, BLUR_RADIUS);
    }

    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    image::ImageEncoder::write_image(encoder, &small, w, h, image::ExtendedColorType::Rgba8).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
}

/// Redução por média de blocos.
fn downscale(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for dy in 0..dh {
        let (y0, y1) = (dy * sh / dh, ((dy + 1) * sh / dh).max(dy * sh / dh + 1));
        for dx in 0..dw {
            let (x0, x1) = (dx * sw / dw, ((dx + 1) * sw / dw).max(dx * sw / dw + 1));
            let mut acc = [0u32; 4];
            for y in y0..y1.min(sh) {
                for x in x0..x1.min(sw) {
                    let i = ((y * sw + x) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += src[i + c] as u32;
                    }
                }
            }
            let n = ((y1.min(sh) - y0) * (x1.min(sw) - x0)).max(1);
            let o = ((dy * dw + dx) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    out
}

/// Desfoque de caixa separável (horizontal e vertical), bordas estendidas.
fn box_blur(px: &mut [u8], w: u32, h: u32, r: i32) {
    let (w, h) = (w as i32, h as i32);
    let mut tmp = px.to_vec();
    let at = |x: i32, y: i32| ((y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) * 4) as usize;
    let n = (2 * r + 1) as u32;
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for k in -r..=r {
                let i = at(x + k, y);
                for c in 0..4 {
                    acc[c] += px[i + c] as u32;
                }
            }
            let o = at(x, y);
            for c in 0..4 {
                tmp[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0u32; 4];
            for k in -r..=r {
                let i = at(x, y + k);
                for c in 0..4 {
                    acc[c] += tmp[i + c] as u32;
                }
            }
            let o = at(x, y);
            for c in 0..4 {
                px[o + c] = (acc[c] / n) as u8;
            }
        }
    }
}

