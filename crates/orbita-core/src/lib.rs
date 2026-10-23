//! Núcleo de conversões do Órbita.
//!
//! Não depende de interface gráfica nem de sistema operacional: o app desktop, a
//! linha de comando e os testes usam o mesmo [`Engine`].

pub mod converters;
pub mod error;
pub mod format;
pub mod job;
pub mod naming;
pub mod pdfium;

use std::path::{Path, PathBuf};

use serde::Serialize;

pub use converters::{Converter, Mode, Output};
pub use error::{ConvertError, Result};
pub use format::Format;
pub use job::{CancelToken, Ctx};
pub use pdfium::PdfiumHost;

/// Uma opção do menu radial para a seleção atual.
#[derive(Debug, Clone, Serialize)]
pub struct ActionInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
}

pub struct Engine {
    pdfium: PdfiumHost,
    converters: Vec<Box<dyn Converter>>,
}

impl Engine {
    /// `pdfium_dirs`: pastas onde procurar a biblioteca pdfium.
    pub fn new(pdfium_dirs: Vec<PathBuf>) -> Self {
        Self { pdfium: PdfiumHost::new(pdfium_dirs), converters: converters::all() }
    }

    /// Opções disponíveis para os arquivos selecionados.
    pub fn actions_for(&self, inputs: &[PathBuf]) -> Vec<ActionInfo> {
        available_actions(&self.converters, inputs)
    }

    /// Executa uma ação. `progress` recebe valores de 0.0 a 1.0.
    pub fn run(
        &self,
        action_id: &str,
        inputs: &[PathBuf],
        progress: &dyn Fn(f32),
        cancel: &CancelToken,
    ) -> Result<Output> {
        let converter = self
            .converters
            .iter()
            .find(|c| c.id() == action_id)
            .ok_or(ConvertError::Unsupported)?;
        if inputs.is_empty() || inputs.iter().any(|p| !p.is_file()) {
            return Err(ConvertError::Other("Arquivo não encontrado".into()));
        }
        let ctx = Ctx::new(&self.pdfium, progress, cancel);

        match converter.mode() {
            Mode::Combine => converter.run(&ctx, inputs),
            Mode::PerFile if inputs.len() == 1 => converter.run(&ctx, inputs),
            Mode::PerFile => run_each(converter.as_ref(), &ctx, inputs),
        }
    }
}

/// Opções disponíveis para os arquivos selecionados, sem precisar de um
/// [`Engine`] (não carrega o pdfium).
pub fn actions_for(inputs: &[PathBuf]) -> Vec<ActionInfo> {
    available_actions(&converters::all(), inputs)
}

fn available_actions(converters: &[Box<dyn Converter>], inputs: &[PathBuf]) -> Vec<ActionInfo> {
    let formats: Option<Vec<Format>> = inputs.iter().map(|p| Format::from_path(p)).collect();
    let Some(formats) = formats.filter(|f| !f.is_empty()) else { return Vec::new() };

    converters
        .iter()
        .filter(|c| match c.mode() {
            Mode::Combine => formats.len() >= 2,
            Mode::PerFile => {
                formats.iter().all(|&f| c.accepts(f))
                    // Não oferece "PNG" quando tudo já é PNG.
                    && !formats.iter().all(|&f| Some(f) == c.output_format())
            }
        })
        .map(|c| ActionInfo { id: c.id(), label: c.label(), icon: c.icon() })
        .collect()
}

/// Aplica um conversor de arquivo único a cada arquivo da seleção. Se alguns
/// falharem, entrega os que deram certo e avisa quantos falharam.
fn run_each(converter: &dyn Converter, ctx: &Ctx, inputs: &[PathBuf]) -> Result<Output> {
    let mut output = Output::default();
    let mut failures = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        ctx.check_cancel()?;
        match converter.run(&ctx.slice(i, inputs.len()), std::slice::from_ref(input)) {
            Ok(out) => {
                output.created.extend(out.created);
                output.notes.extend(out.notes);
            }
            Err(ConvertError::Cancelled) => return Err(ConvertError::Cancelled),
            Err(e) => failures.push((display_name(input), e)),
        }
    }
    match (output.created.is_empty(), failures.is_empty()) {
        (true, false) => Err(failures.remove(0).1),
        _ => {
            if let Some((name, e)) = failures.first() {
                output.notes.push(format!("{} de {} falharam (ex.: {name}: {e})", failures.len(), inputs.len()));
            }
            Ok(output)
        }
    }
}

fn display_name(path: &Path) -> String {
    path.file_name().unwrap_or_default().to_string_lossy().into_owned()
}
