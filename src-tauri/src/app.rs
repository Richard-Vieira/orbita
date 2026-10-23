//! Montagem do app: plugins, janelas, bandeja, atalho global e comandos
//! chamados pela interface.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::files::existing_files;
use crate::jobs::Jobs;
use crate::menu::{self, MenuState};
use crate::{registration, toast, tray};

/// Atalho global padrão.
pub const SHORTCUT: &str = "Ctrl+Alt+Space";
const SHORTCUT_TEXT: &str = "Ctrl+Alt+Espaço";

pub fn run(args: Vec<String>) {
    let started_in_background = args.iter().any(|a| a == "--background");
    let initial_files = existing_files(&args);
    let args_for_log = args.clone();

    tauri::Builder::default()
        // Uma única instância: se o Órbita já estiver aberto, os argumentos
        // (arquivos) são repassados para ele.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let files = existing_files(argv.iter().skip(1));
            if files.is_empty() {
                show_help(app);
            } else {
                open_menu(app, files, None);
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        // Lê a seleção do Explorer fora da thread principal.
                        std::thread::spawn(move || {
                            let files = orbita_platform::selected_files().unwrap_or_default();
                            menu::open(&app, files, None);
                        });
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![ui_ready, ui_log, run_action, menu_for, hide_menu, hide_toast, reveal])
        .setup(move |app| {
            let handle = app.handle().clone();
            crate::logger::init(handle.path().app_log_dir().ok());
            log::info!("iniciando (args: {args_for_log:?})");
            app.manage(Jobs::start(handle.clone(), pdfium_dirs(&handle)));
            // As janelas são criadas só no primeiro uso (veja lifecycle.rs).
            tray::create(&handle)?;

            if let Err(e) = registration::ensure_shell_integration() {
                log::error!("menu de contexto: {e}");
            }
            start_shell_server(&handle);

            if let Err(e) = app.global_shortcut().register(SHORTCUT) {
                log::warn!("atalho {SHORTCUT}: {e}");
                toast::info(&handle, "Atalho indisponível", &format!("{SHORTCUT_TEXT} já é usado por outro programa."));
            }

            if first_run(&handle) {
                // Versões de desenvolvimento não se cadastram para iniciar com o Windows.
                if !cfg!(debug_assertions) {
                    let _ = registration::set_autostart(true);
                }
                show_help(&handle);
            } else if !initial_files.is_empty() {
                open_menu(&handle, initial_files.clone(), None);
            } else if !started_in_background {
                show_help(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Órbita")
        .run(|_app, event| {
            // O Órbita fica na bandeja mesmo sem janelas abertas (elas são
            // descartadas quando ocioso). Só sai pelo "Sair" da bandeja.
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}

/// Abre o menu fora da thread que recebeu o pedido (a captura do fundo e o
/// posicionamento não podem atrasar o Explorer nem o laço de eventos).
fn open_menu(app: &AppHandle, files: Vec<PathBuf>, point: Option<(i32, i32)>) {
    let app = app.clone();
    std::thread::spawn(move || menu::open(&app, files, point));
}

fn start_shell_server(app: &AppHandle) {
    let handle = app.clone();
    let result = orbita_platform::shell::serve(move |invocation| {
        log::info!("menu de contexto: {} arquivo(s), ponto {:?}", invocation.files.len(), invocation.point);
        open_menu(&handle, invocation.files, invocation.point);
    });
    if let Err(e) = result {
        log::error!("servidor do menu de contexto: {e}");
    }
}

pub fn show_help(app: &AppHandle) {
    toast::info(
        app,
        "Órbita está ativo",
        &format!("Botão direito num arquivo › Converter com Órbita, ou {SHORTCUT_TEXT}."),
    );
}

/// Na primeira execução cria um marcador na pasta de dados do app.
fn first_run(app: &AppHandle) -> bool {
    let Ok(dir) = app.path().app_config_dir() else { return false };
    let marker = dir.join("instalado");
    if marker.exists() {
        return false;
    }
    let _ = std::fs::create_dir_all(&dir);
    std::fs::write(&marker, b"").is_ok()
}

/// Onde procurar o pdfium: pasta de recursos do app instalado, pasta do
/// executável e, em desenvolvimento, vendor/ do repositório.
fn pdfium_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        dirs.push(dir);
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        dirs.push(dir);
    }
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vendor/pdfium/bin"));
    }
    dirs
}

// ---------------------------------------------------------------------------
// Comandos chamados pela interface
// ---------------------------------------------------------------------------

/// A página de uma janela terminou de carregar.
#[tauri::command]
fn ui_ready(app: AppHandle, window: tauri::WebviewWindow) {
    match window.label() {
        menu::LABEL if menu::GATE.mark_ready(&app) => menu::show(&app),
        menu::LABEL => log::info!("menu pronto"),
        toast::LABEL if toast::GATE.mark_ready(&app) => toast::show_window(&app),
        _ => {}
    }
}

/// Mensagens de diagnóstico da interface (vão para o log).
#[tauri::command]
fn ui_log(window: tauri::WebviewWindow, message: String) {
    log::info!("[{}] {message}", window.label());
}

#[tauri::command]
fn run_action(jobs: State<'_, Jobs>, action: String, files: Vec<String>) -> u64 {
    jobs.submit(action, files.into_iter().map(PathBuf::from).collect())
}

/// Estado do menu para outro arquivo (encadear conversões).
#[tauri::command]
fn menu_for(files: Vec<String>) -> MenuState {
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    MenuState::for_files(&files)
}

#[tauri::command]
fn hide_menu(app: AppHandle) {
    menu::hide(&app);
}

#[tauri::command]
fn hide_toast(app: AppHandle) {
    toast::hide(&app);
}

#[tauri::command]
fn reveal(path: String) -> Result<(), String> {
    orbita_platform::reveal(std::path::Path::new(&path)).map_err(|e| e.to_string())
}
