//! Linha de comando do Órbita.
//!
//!     orbita-cli acoes <arquivo>...
//!     orbita-cli converter <acao> <arquivo>...
//!
//! Usada nos testes, no CI e (futuramente) pelas integrações com gerenciadores de
//! arquivos do Linux.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use orbita_core::{CancelToken, Engine};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let engine = Engine::new(pdfium_dirs());

    match args.split_first() {
        Some((cmd, files)) if cmd == "acoes" && !files.is_empty() => {
            let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
            for action in engine.actions_for(&files) {
                println!("{:<14} {}", action.id, action.label);
            }
            ExitCode::SUCCESS
        }
        Some((cmd, rest)) if cmd == "converter" && rest.len() >= 2 => {
            let action = &rest[0];
            let files: Vec<PathBuf> = rest[1..].iter().map(PathBuf::from).collect();
            let start = Instant::now();
            let progress = |p: f32| eprint!("\r{:>3.0}%", p * 100.0);
            match engine.run(action, &files, &progress, &CancelToken::default()) {
                Ok(out) => {
                    eprintln!("\rconcluído em {:.2}s", start.elapsed().as_secs_f32());
                    for path in out.created {
                        println!("{}", path.display());
                    }
                    for note in out.notes {
                        eprintln!("aviso: {note}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("\rerro: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("uso:\n  orbita-cli acoes <arquivo>...\n  orbita-cli converter <acao> <arquivo>...");
            ExitCode::from(2)
        }
    }
}

/// Procura o pdfium ao lado do executável e na pasta `vendor/` do repositório.
fn pdfium_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        dirs.push(dir);
    }
    let vendor = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/pdfium");
    dirs.push(vendor.join("bin"));
    dirs.push(vendor.join("lib"));
    dirs
}
