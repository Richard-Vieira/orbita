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

fn report(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("orbita: {e}");
            ExitCode::FAILURE
        }
    }
}
