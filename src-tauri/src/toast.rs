//! Notificação discreta no canto inferior direito, com o mesmo visual do menu.
//! A janela não recebe foco, para não fechar o menu que estiver aberto.

use serde::Serialize;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::backdrop;
use crate::gate::Gate;

pub const LABEL: &str = "toast";
pub static GATE: Gate = Gate::new(LABEL);
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 100.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastState {
    pub kind: &'static str,
    pub title: String,
    pub detail: String,
    pub notes: Vec<String>,
    pub path: Option<String>,
    pub backdrop: Option<String>,
}

/// Cria a janela (escondida) se ela ainda não existir.
fn ensure(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(LABEL) {
        return Some(window);
    }
    create(app).map_err(|e| log::error!("não foi possível criar a notificação: {e}")).ok()
}

fn create(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("toast.html".into()))
        .title("Órbita")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .focusable(false)
        .build()
}

pub fn success(app: &AppHandle, title: &str, detail: &str, notes: Vec<String>, path: Option<String>) {
    show(app, ToastState { kind: "success", title: title.into(), detail: detail.into(), notes, path, backdrop: None });
}

pub fn error(app: &AppHandle, title: &str, detail: &str) {
    show(app, ToastState { kind: "error", title: title.into(), detail: detail.into(), notes: Vec::new(), path: None, backdrop: None });
}

pub fn info(app: &AppHandle, title: &str, detail: &str) {
    show(app, ToastState { kind: "info", title: title.into(), detail: detail.into(), notes: Vec::new(), path: None, backdrop: None });
}

fn show(app: &AppHandle, mut state: ToastState) {
    crate::lifecycle::touch(app);
    let Some(window) = ensure(app) else { return };
    let (cx, cy) = orbita_platform::cursor_position().unwrap_or((0, 0));
    let monitor = app
        .monitor_from_point(cx as f64, cy as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };

    let scale = monitor.scale_factor();
    let (w, h) = ((WIDTH * scale).round() as i32, (HEIGHT * scale).round() as i32);
    let area = monitor.work_area();
    let x = area.position.x + area.size.width as i32 - w;
    let y = area.position.y + area.size.height as i32 - h;

    if !window.is_visible().unwrap_or(false) {
        state.backdrop = backdrop::capture(x, y, w as u32, h as u32);
    }
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_size(PhysicalSize::new(w as u32, h as u32));
    if GATE.deliver(app, "toast:show", &state) {
        show_window(app);
    }
}

/// Mostra a notificação sem tirar o foco do menu (senão ele fecharia).
pub fn show_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else { return };
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        if orbita_platform::show_without_focus(hwnd.0 as isize) {
            return;
        }
    }
    let _ = window.show();
    // Fora do Windows o show() pega o foco: devolve para o menu aberto, que
    // fecharia ao perdê-lo.
    #[cfg(not(windows))]
    if crate::menu::is_visible(app) {
        if let Some(menu) = app.get_webview_window(crate::menu::LABEL) {
            let _ = menu.set_focus();
        }
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}
