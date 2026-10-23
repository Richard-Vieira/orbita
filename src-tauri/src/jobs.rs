//! Fila de conversões: uma thread dedicada executa uma conversão por vez
//! (o pdfium não é thread-safe) sem nunca travar a interface.

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;

use orbita_core::{CancelToken, Engine};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::files::FileInfo;
use crate::{menu, toast};

struct Job {
    id: u64,
    action: String,
    files: Vec<PathBuf>,
}

pub struct Jobs {
    sender: Mutex<Sender<Job>>,
    next_id: AtomicU64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    job_id: u64,
    progress: f32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Done {
    job_id: u64,
    created: Vec<FileInfo>,
    notes: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Failed {
    job_id: u64,
    message: String,
}

impl Jobs {
    pub fn start(app: AppHandle, pdfium_dirs: Vec<PathBuf>) -> Jobs {
        let (sender, receiver) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("orbita-jobs".into())
            .spawn(move || {
                let engine = Engine::new(pdfium_dirs);
                for job in receiver {
                    run(&app, &engine, job);
                }
            })
            .expect("thread de conversões");
        Jobs { sender: Mutex::new(sender), next_id: AtomicU64::new(1) }
    }

    pub fn submit(&self, action: String, files: Vec<PathBuf>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let sender = self.sender.lock().unwrap_or_else(|e| e.into_inner());
        let _ = sender.send(Job { id, action, files });
        id
    }
}

fn run(app: &AppHandle, engine: &Engine, job: Job) {
    let last = std::cell::Cell::new(-1.0f32);
    let progress = |p: f32| {
        if p - last.get() >= 0.02 || p >= 1.0 {
            last.set(p);
            let _ = app.emit_to(menu::LABEL, "job:progress", Progress { job_id: job.id, progress: p });
        }
    };

    // Um erro inesperado numa conversão nunca derruba o app.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        engine.run(&job.action, &job.files, &progress, &CancelToken::default())
    }))
    .unwrap_or_else(|_| Err(orbita_core::ConvertError::Other("Erro inesperado na conversão".into())));

    match result {
        Ok(output) => {
            let created: Vec<FileInfo> = output.created.iter().map(|p| FileInfo::from_path(p)).collect();
            let _ = app.emit_to(
                menu::LABEL,
                "job:done",
                Done { job_id: job.id, created: created.clone(), notes: output.notes.clone() },
            );
            if let Some(first) = created.first() {
                let title = match (created.len(), first.is_dir) {
                    (1, true) => "Pasta criada".to_string(),
                    (1, false) => "Arquivo criado".to_string(),
                    (n, _) => format!("{n} arquivos criados"),
                };
                toast::success(app, &title, &first.name, output.notes, Some(first.path.clone()));
            }
        }
        Err(e) => {
            let message = e.to_string();
            log::warn!("conversão {} falhou: {message}", job.action);
            let _ = app.emit_to(menu::LABEL, "job:error", Failed { job_id: job.id, message: message.clone() });
            if !menu::is_visible(app) {
                toast::error(app, "Não foi possível converter", &message);
            }
        }
    }
}
