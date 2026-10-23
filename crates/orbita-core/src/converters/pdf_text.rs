//! PDF -> TXT (UTF-8).

use std::fs;
use std::path::PathBuf;

use super::{single_input, Converter, Output};
use crate::error::{ConvertError, Result};
use crate::format::Format;
use crate::job::Ctx;
use crate::naming::{map_io, parent_dir, stem, unique_path, write_atomic};

pub struct PdfToText;

impl Converter for PdfToText {
    fn id(&self) -> &'static str {
        "pdf.txt"
    }
    fn label(&self) -> &'static str {
        "Texto"
    }
    fn icon(&self) -> &'static str {
        "txt"
    }
    fn accepts(&self, format: Format) -> bool {
        format == Format::Pdf
    }

    fn run(&self, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
        let input = single_input(inputs);
        let pdfium = ctx.pdfium.get()?;
        let doc = pdfium.load_pdf_from_file(input, None)?;
        let pages = doc.pages();
        let count = pages.len().max(1) as f32;

        let mut text = String::new();
        for (i, page) in pages.iter().enumerate() {
            ctx.check_cancel()?;
            let raw = page.text()?.all();
            if i > 0 {
                text.push_str("\n\n");
            }
            text.push_str(clean(&raw).trim_end());
            ctx.progress((i + 1) as f32 / count * 0.9);
        }
        if text.trim().is_empty() {
            return Err(ConvertError::NoText);
        }
        text.push('\n');
        if cfg!(windows) {
            text = text.replace('\n', "\r\n");
        }

        let target = unique_path(&parent_dir(input), &stem(input), "txt");
        write_atomic(&target, |tmp| fs::write(tmp, &text).map_err(|e| map_io(e, tmp)))?;
        ctx.progress(1.0);
        Ok(Output::single(target))
    }
}

/// Limpa o texto extraído pelo pdfium:
/// - o pdfium marca hifenização de fim de linha com U+0002 ("apren\u{2}\r\ndizado");
/// - normaliza quebras de linha;
/// - desfaz ligaduras tipográficas (ﬁ, ﬂ...).
fn clean(raw: &str) -> String {
    let mut s = raw.replace("\r\n", "\n").replace('\r', "\n");
    s = s.replace("\u{2}\n", "").replace('\u{2}', "-");
    s.chars()
        .flat_map(|c| -> Vec<char> {
            match c {
                '\u{FB00}' => vec!['f', 'f'],
                '\u{FB01}' => vec!['f', 'i'],
                '\u{FB02}' => vec!['f', 'l'],
                '\u{FB03}' => vec!['f', 'f', 'i'],
                '\u{FB04}' => vec!['f', 'f', 'l'],
                '\u{FB05}' | '\u{FB06}' => vec!['s', 't'],
                '\u{0}' => vec![],
                c => vec![c],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn joins_hyphenated_words_and_ligatures() {
        assert_eq!(clean("apren\u{2}\r\ndizado ﬁnal"), "aprendizado final");
    }
}
