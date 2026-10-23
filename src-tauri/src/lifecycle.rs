//! Ciclo de vida das janelas, para o Órbita ser leve em segundo plano.
//!
//! As janelas (e o motor WebView2 que elas carregam) só existem enquanto o
//! Órbita está sendo usado: são criadas no primeiro uso, ficam prontas por
//! alguns minutos para abrir na hora e depois são descartadas, voltando o app
//! ao consumo mínimo de memória.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::jobs::Jobs;
use crate::{menu, toast};

/// Quanto tempo as janelas ficam prontas depois do último uso (padrão: 10 min;
/// `ORBITA_KEEP_WARM_SECS` muda isso, útil para testes).
fn keep_warm() -> Duration {
    let secs = std::env::var("ORBITA_KEEP_WARM_SECS").ok().and_then(|v| v.parse().ok());
    Duration::from_secs(secs.unwrap_or(10 * 60))
}

static LAST_USE: AtomicU64 = AtomicU64::new(0);

/// Registra um uso e agenda o descarte das janelas para depois do período de espera.
pub fn touch(app: &AppHandle) {
    let generation = LAST_USE.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(keep_warm());
        let idle = LAST_USE.load(Ordering::SeqCst) == generation
            && !menu::is_visible(&app)
            && app.try_state::<Jobs>().is_none_or(|jobs| jobs.is_idle());
        if idle {
            release(&app);
        }
    });
}

/// Fecha as janelas e libera o motor do WebView2.
pub fn release(app: &AppHandle) {
    for label in [menu::LABEL, toast::LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.destroy();
        }
    }
    menu::GATE.reset();
    toast::GATE.reset();
    log::info!("janelas liberadas (inativo)");
}
