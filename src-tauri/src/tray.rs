//! Icono de la bandeja: ajustes, cambiar de monitor, ocultar y salir.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Ajustes…", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next_monitor", "Siguiente monitor", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "toggle_hidden", "Ocultar / mostrar isla", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&settings, &next, &hide, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Dynamic Island")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => crate::open_settings_window(app),
            "next_monitor" => crate::window::next_monitor(app),
            "toggle_hidden" => crate::window::toggle_hidden(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
