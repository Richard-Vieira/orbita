//! Ícone na bandeja do sistema.

use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::{app, registration, toast};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let title = MenuItemBuilder::with_id("title", "Órbita").enabled(false).build(app)?;
    let help = MenuItemBuilder::with_id("help", "Como usar").build(app)?;
    let autostart = CheckMenuItemBuilder::with_id("autostart", if cfg!(windows) { "Iniciar com o Windows" } else { "Iniciar com o sistema" })
        .checked(registration::autostart_enabled())
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Sair").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&title)
        .separator()
        .item(&help)
        .item(&autostart)
        .separator()
        .item(&quit)
        .build()?;

    let mut tray = TrayIconBuilder::with_id("orbita")
        .tooltip("Órbita")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "help" => app::show_help(app),
            "autostart" => {
                let enable = !registration::autostart_enabled();
                if let Err(e) = registration::set_autostart(enable) {
                    toast::error(app, "Não foi possível alterar", &e);
                }
                let _ = autostart.set_checked(registration::autostart_enabled());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                app::show_help(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
