//! Nomes dos arquivos gerados: nunca sobrescreve nada, usa sufixos (1), (2)...

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ConvertError, Result};

/// Nome base do arquivo, sem extensão.
pub fn stem(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_else(|| OsStr::new("arquivo"))
        .to_string_lossy()
        .into_owned()
}

/// Pasta onde o arquivo novo será criado (a mesma do original).
pub fn parent_dir(path: &Path) -> PathBuf {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Primeiro caminho livre para `dir/base.ext`, `dir/base (1).ext`, `dir/base (2).ext`...
/// Com `ext` vazio, gera nomes de pasta.
pub fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let make = |n: u32| {
        let name = match (n, ext.is_empty()) {
            (0, true) => base.to_string(),
            (0, false) => format!("{base}.{ext}"),
            (n, true) => format!("{base} ({n})"),
            (n, false) => format!("{base} ({n}).{ext}"),
        };
        dir.join(name)
    };
    (0..)
        .map(make)
        .find(|p| !p.exists())
        .expect("sempre existe um nome livre")
}

/// Escreve um arquivo de forma atômica: grava num temporário oculto na mesma pasta
/// e só renomeia para o nome final quando terminou sem erro. Assim o usuário nunca
/// vê um arquivo pela metade e o original nunca é tocado.
pub fn write_atomic<F>(target: &Path, write: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let dir = parent_dir(target);
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let tmp = dir.join(format!(".{name}.orbita-tmp"));
    let result = write(&tmp).and_then(|_| {
        fs::rename(&tmp, target).map_err(|e| map_io(e, target))
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Converte erros de E/S de permissão/arquivo bloqueado numa mensagem amigável.
pub fn map_io(e: std::io::Error, path: &Path) -> ConvertError {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => ConvertError::Access(path.to_path_buf()),
        _ if e.raw_os_error() == Some(32) => ConvertError::Access(path.to_path_buf()), // ERROR_SHARING_VIOLATION
        _ => ConvertError::Io(e),
    }
}

/// Ordenação "natural" de nomes: "página 2" vem antes de "página 10".
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (na, nb) = (take(&mut a), take(&mut b));
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_lowercase().cmp(y.to_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                a.next();
                b.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_adds_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let first = unique_path(dir.path(), "foto", "png");
        assert_eq!(first.file_name().unwrap(), "foto.png");
        fs::write(&first, b"x").unwrap();
        let second = unique_path(dir.path(), "foto", "png");
        assert_eq!(second.file_name().unwrap(), "foto (1).png");
        fs::write(&second, b"x").unwrap();
        assert_eq!(unique_path(dir.path(), "foto", "png").file_name().unwrap(), "foto (2).png");
    }

    #[test]
    fn natural_order() {
        let mut v = vec!["a10.pdf", "a2.pdf", "A1.pdf", "b.pdf"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["A1.pdf", "a2.pdf", "a10.pdf", "b.pdf"]);
    }

    #[test]
    fn atomic_cleans_up_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("x.txt");
        let r = write_atomic(&target, |tmp| {
            fs::write(tmp, b"meio")?;
            Err(ConvertError::Cancelled)
        });
        assert!(r.is_err());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
