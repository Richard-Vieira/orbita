// Sem janela de console no executável final.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backdrop;
mod files;
mod gate;
mod jobs;
mod lifecycle;
mod logger;
mod menu;
mod registration;
mod toast;
mod tray;

use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    linux_env();

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Chamados pelo instalador/desinstalador: fazem o trabalho e saem, sem
    // abrir interface nem falar com a instância que já estiver rodando.
    if args.iter().any(|a| a == "--register") {
        return report(registration::install());
    }
    if args.iter().any(|a| a == "--unregister") {
        return report(registration::uninstall());
    }

    app::run(args);
    ExitCode::SUCCESS
}

/// No Wayland o app não consegue posicionar a janela no mouse nem ficar por
/// cima: roda pelo XWayland. E o renderizador DMA-BUF do WebKit deixa a janela
/// em branco em algumas placas (NVIDIA). Quem já definiu as variáveis manda.
#[cfg(target_os = "linux")]
fn linux_env() {
    if std::env::var_os("GDK_BACKEND").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

fn report(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("orbita: {e}");
            ExitCode::FAILURE
        }
    }
}
