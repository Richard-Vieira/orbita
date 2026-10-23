//! Ciclo de vida das janelas, para o Órbita ser leve em segundo plano.
//!
//! As janelas (e o motor WebView2 que elas carregam) só existem enquanto o
//! Órbita está sendo usado: são criadas no primeiro uso, ficam prontas por
//! alguns minutos para abrir na hora e depois são descartadas, voltando o app
//! ao consumo mínimo de memória.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

use crate::jobs::Jobs;
use crate::{menu, toast};

/// De quanto em quanto tempo o vigia confere se pode liberar as janelas.
const CHECK_EVERY: Duration = Duration::from_secs(30);

static LAST_USE: Mutex<Option<Instant>> = Mutex::new(None);
static WATCHING: AtomicBool = AtomicBool::new(false);

/// Quanto tempo as janelas ficam prontas depois do último uso (padrão: 10 min;
/// `ORBITA_KEEP_WARM_SECS` muda isso, útil para testes).
fn keep_warm() -> Duration {
    let secs = std::env::var("ORBITA_KEEP_WARM_SECS").ok().and_then(|v| v.parse().ok());
    Duration::from_secs(secs.unwrap_or(10 * 60))
}

/// Registra um uso (menu aberto, notificação, conversão terminada).
pub fn touch(app: &AppHandle) {
    *LAST_USE.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    if !WATCHING.swap(true, Ordering::SeqCst) {
        let app = app.clone();
        std::thread::Builder::new()
            .name("orbita-lifecycle".into())
            .spawn(move || watch(app))
            .ok();
    }
}

/// Uma única thread que acorda de tempos em tempos e libera as janelas quando
/// o app está ocioso há tempo suficiente.
fn watch(app: AppHandle) {
    let limit = keep_warm();
    let tick = CHECK_EVERY.min(limit);
    loop {
        std::thread::sleep(tick);
        let last = *LAST_USE.lock().unwrap_or_else(|e| e.into_inner());
        let expired = last.is_some_and(|t| t.elapsed() >= limit);
        let busy = menu::is_visible(&app) || app.try_state::<Jobs>().is_some_and(|jobs| !jobs.is_idle());
        if expired && !busy && has_windows(&app) {
            release(&app);
        }
    }
}

fn has_windows(app: &AppHandle) -> bool {
    app.get_webview_window(menu::LABEL).is_some() || app.get_webview_window(toast::LABEL).is_some()
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
