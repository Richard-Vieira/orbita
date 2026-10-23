//! Log simples em arquivo (%LOCALAPPDATA%\<identificador>\logs\orbita.log),
//! útil para quem for reportar um problema. Nunca registra conteúdo de arquivos.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use log::{Level, LevelFilter, Metadata, Record};

const MAX_BYTES: u64 = 512 * 1024;

struct FileLogger {
    file: Mutex<Option<File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) || !record.target().starts_with("orbita") {
            return;
        }
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let line = format!("{millis} {:<5} {}\n", record.level(), record.args());
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        if let Some(file) = self.file.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {}
}

pub fn init(dir: Option<PathBuf>) {
    let file = dir.and_then(|dir| {
        fs::create_dir_all(&dir).ok()?;
        let path = dir.join("orbita.log");
        if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
            let _ = fs::rename(&path, dir.join("orbita.old.log"));
        }
        OpenOptions::new().create(true).append(true).open(path).ok()
    });
    let logger = Box::leak(Box::new(FileLogger { file: Mutex::new(file) }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
