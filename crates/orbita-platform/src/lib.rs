//! Integração do Órbita com o sistema operacional.
//!
//! A API pública é a mesma em todos os sistemas; cada um implementa o que
//! consegue. No Linux (ainda não implementado) as funções retornam
//! `Unsupported`/`None` e o app usa alternativas.

use std::path::{Path, PathBuf};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use crate::windows as imp;

#[cfg(not(windows))]
mod linux;
#[cfg(not(windows))]
use crate::linux as imp;

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("recurso não suportado neste sistema")]
    Unsupported,
    #[error("{0}")]
    Os(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Pedido de abertura do menu vindo do gerenciador de arquivos.
#[derive(Debug, Clone)]
pub struct Invocation {
    pub files: Vec<PathBuf>,
    /// Ponto do clique em coordenadas físicas de tela, se o sistema informar.
    pub point: Option<(i32, i32)>,
}

/// Imagem RGBA capturada da tela.
pub struct Capture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Item "Converter com Órbita" no menu do botão direito.
pub mod shell {
    use super::*;

    /// Registra (ou atualiza) o item do menu para as extensões dadas, apontando
    /// para `exe`. Não exige administrador.
    pub fn register(exe: &Path, extensions: &[&str]) -> Result<()> {
        imp::shell::register(exe, extensions)
    }

    /// Remove tudo que `register` criou.
    pub fn unregister(extensions: &[&str]) -> Result<()> {
        imp::shell::unregister(extensions)
    }

    /// Começa a receber os pedidos do menu de contexto. O callback é chamado
    /// numa thread própria e deve retornar rápido.
    pub fn serve(on_invoke: impl Fn(Invocation) + Send + Sync + 'static) -> Result<()> {
        imp::shell::serve(Box::new(on_invoke))
    }
}

/// Abrir o Órbita automaticamente ao entrar no sistema.
pub mod autostart {
    use super::*;

    pub fn set(name: &str, exe: &Path, enabled: bool) -> Result<()> {
        imp::autostart::set(name, exe, enabled)
    }

    pub fn is_enabled(name: &str) -> bool {
        imp::autostart::is_enabled(name)
    }
}

/// Arquivos selecionados no gerenciador de arquivos em primeiro plano
/// (ou na área de trabalho). Vazio se não houver seleção.
pub fn selected_files() -> Result<Vec<PathBuf>> {
    imp::selected_files()
}

/// Posição do mouse em pixels físicos.
pub fn cursor_position() -> Option<(i32, i32)> {
    imp::cursor_position()
}

/// Captura um retângulo da tela (pixels físicos). Usado para o efeito de vidro.
/// A imagem fica só na memória.
pub fn capture_screen(x: i32, y: i32, width: u32, height: u32) -> Option<Capture> {
    imp::capture_screen(x, y, width, height)
}

/// Abre a pasta do arquivo com ele selecionado.
pub fn reveal(path: &Path) -> Result<()> {
    imp::reveal(path)
}

/// Mostra uma janela sem tirar o foco da janela atual (usado na notificação,
/// para não fechar o menu). `handle` é o HWND no Windows. Retorna `false` se o
/// sistema não suportar; aí o app usa o `show()` normal.
pub fn show_without_focus(handle: isize) -> bool {
    imp::show_without_focus(handle)
}

/// `false` quando o usuário desligou os efeitos de transparência do sistema.
pub fn transparency_enabled() -> bool {
    imp::transparency_enabled()
}
