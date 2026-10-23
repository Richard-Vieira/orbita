//! Entrega de eventos para janelas que ainda estão carregando.
//!
//! As janelas são criadas escondidas na inicialização, mas a página leva alguns
//! instantes para carregar. Um pedido que chega antes disso (ex.: o Explorer
//! abre o Órbita já com um arquivo) fica guardado aqui e é entregue assim que a
//! página avisa que está pronta (`ui_ready`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub struct Gate {
    label: &'static str,
    ready: AtomicBool,
    pending: Mutex<Option<(&'static str, serde_json::Value)>>,
}

impl Gate {
    pub const fn new(label: &'static str) -> Gate {
        Gate { label, ready: AtomicBool::new(false), pending: Mutex::new(None) }
    }

    /// Emite o evento se a página estiver pronta e retorna `true`; senão guarda
    /// (substituindo o pedido anterior) e retorna `false`.
    pub fn deliver<T: Serialize>(&self, app: &AppHandle, event: &'static str, payload: &T) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if self.ready.load(Ordering::Acquire) {
            let _ = app.emit_to(self.label, event, payload);
            true
        } else {
            *pending = serde_json::to_value(payload).ok().map(|v| (event, v));
            false
        }
    }

    /// Marca a página como pronta e devolve `true` se havia algo guardado
    /// (que já foi emitido).
    pub fn mark_ready(&self, app: &AppHandle) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        self.ready.store(true, Ordering::Release);
        match pending.take() {
            Some((event, payload)) => {
                let _ = app.emit_to(self.label, event, payload);
                true
            }
            None => false,
        }
    }
}
