//! Janela do menu radial: criada escondida na inicialização (abre na hora) e
//! posicionada centralizada no mouse a cada uso.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::backdrop;
use crate::files::FileInfo;
use crate::gate::Gate;
use crate::toast;

pub const LABEL: &str = "menu";
/// Tamanho da janela em pixels lógicos (precisa bater com ui/src/geometry.ts).
const SIZE: f64 = 560.0;

pub static GATE: Gate = Gate::new(LABEL);

/// Evita duas aberturas simultâneas (atalho + menu de contexto ao mesmo tempo).
static OPENING: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuState {
    pub files: Vec<FileInfo>,
    pub actions: Vec<orbita_core::ActionInfo>,
    pub backdrop: Option<String>,
    pub glass: bool,
}

impl MenuState {
    pub fn for_files(files: &[PathBuf]) -> MenuState {
        MenuState {
            files: files.iter().map(|p| FileInfo::from_path(p)).collect(),
            actions: orbita_core::actions_for(files),
            backdrop: None,
            glass: orbita_platform::transparency_enabled(),
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Órbita")
        .inner_size(SIZE, SIZE)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(true)
        .build()?;
    Ok(())
}

/// Abre o menu para os arquivos dados, no ponto dado (ou na posição do mouse).
pub fn open(app: &AppHandle, files: Vec<PathBuf>, point: Option<(i32, i32)>) {
    let _guard = OPENING.lock().unwrap_or_else(|e| e.into_inner());
    let files: Vec<PathBuf> = files.into_iter().filter(|p| p.is_file()).collect();
    if files.is_empty() {
        toast::info(app, "Nenhum arquivo selecionado", "Selecione um arquivo no Explorer ou na área de trabalho.");
        return;
    }
    let Some(window) = app.get_webview_window(LABEL) else { return };

    // Se já estiver aberto, esconde antes de capturar o fundo (senão o
    // próprio menu apareceria na captura).
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        std::thread::sleep(std::time::Duration::from_millis(60));
    }

    let (cx, cy) = point.or_else(orbita_platform::cursor_position).unwrap_or((0, 0));
    let monitor = app
        .monitor_from_point(cx as f64, cy as f64)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let (scale, area) = match &monitor {
        Some(m) => (m.scale_factor(), Some(*m.work_area())),
        None => (1.0, None),
    };
    let size = (SIZE * scale).round() as i32;
    let (mut x, mut y) = (cx - size / 2, cy - size / 2);
    if let Some(area) = area {
        let (ax, ay) = (area.position.x, area.position.y);
        let (aw, ah) = (area.size.width as i32, area.size.height as i32);
        x = x.clamp(ax, (ax + aw - size).max(ax));
        y = y.clamp(ay, (ay + ah - size).max(ay));
    }

    let mut state = MenuState::for_files(&files);
    state.backdrop = backdrop::capture(x, y, size as u32, size as u32);

    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_size(PhysicalSize::new(size as u32, size as u32));
    let delivered = GATE.deliver(app, "menu:open", &state);
    log::info!(
        "abrindo menu em ({x}, {y}) tamanho {size}, {} ação(ões), fundo {}, página pronta {delivered}",
        state.actions.len(),
        state.backdrop.is_some()
    );
    if delivered {
        show(app);
    }
}

/// Mostra a janela e tenta trazê-la para a frente com o foco.
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        if let Err(e) = window.set_focus() {
            log::warn!("menu sem foco: {e}");
        }
    }
}

pub fn hide(app: &AppHandle) {
    log::info!("menu fechado");
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

pub fn is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}
