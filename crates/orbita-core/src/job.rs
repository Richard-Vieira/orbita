use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::{ConvertError, Result};
use crate::pdfium::PdfiumHost;

/// Sinal de cancelamento compartilhado entre a interface e a conversão.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Contexto passado a cada conversor: acesso ao pdfium, progresso e cancelamento.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub pdfium: &'a PdfiumHost,
    progress: &'a dyn Fn(f32),
    cancel: &'a CancelToken,
    /// Faixa [início, fim) do progresso total reservada para esta etapa.
    span: (f32, f32),
}

impl<'a> Ctx<'a> {
    pub fn new(pdfium: &'a PdfiumHost, progress: &'a dyn Fn(f32), cancel: &'a CancelToken) -> Self {
        Self { pdfium, progress, cancel, span: (0.0, 1.0) }
    }

    /// Informa o progresso desta etapa, de 0.0 a 1.0.
    pub fn progress(&self, fraction: f32) {
        let (a, b) = self.span;
        (self.progress)(a + (b - a) * fraction.clamp(0.0, 1.0));
    }

    /// Retorna erro se o usuário cancelou.
    pub fn check_cancel(&self) -> Result<()> {
        if self.cancel.is_cancelled() {
            Err(ConvertError::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Sub-contexto que ocupa a fração [i/n, (i+1)/n) do progresso atual.
    pub fn slice(&self, i: usize, n: usize) -> Ctx<'a> {
        let (a, b) = self.span;
        let w = (b - a) / n.max(1) as f32;
        Ctx { span: (a + w * i as f32, a + w * (i + 1) as f32), ..*self }
    }
}
