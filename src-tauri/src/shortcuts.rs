//! Atajos globales configurables. Se re-registran cada vez que cambian los ajustes.

use crate::settings::Shortcuts;
use crate::Shared;
use std::str::FromStr;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    ToggleExpand,
    NextMonitor,
    ToggleHidden,
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new().with_handler(on_shortcut).build()
}

fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    #[cfg(debug_assertions)]
    eprintln!("[atajo] {shortcut:?} {:?}", event.state);
    if event.state != ShortcutState::Pressed {
        return;
    }
    let action = {
        let st = app.state::<Shared>();
        let bound = st.shortcuts.lock().unwrap();
        bound.iter().find(|(s, _)| s.id() == shortcut.id()).map(|(_, a)| *a)
    };
    match action {
        Some(Action::ToggleExpand) => {
            let _ = app.emit("island://toggle", ());
        }
        Some(Action::NextMonitor) => crate::window::next_monitor(app),
        Some(Action::ToggleHidden) => crate::window::toggle_hidden(app),
        None => {}
    }
}

/// Registra los atajos. Devuelve los que no se pudieron interpretar o registrar.
pub fn register(app: &AppHandle, cfg: &Shortcuts) -> Vec<String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut bound = Vec::new();
    let mut errors = Vec::new();
    for (text, action) in [
        (&cfg.toggle_expand, Action::ToggleExpand),
        (&cfg.next_monitor, Action::NextMonitor),
        (&cfg.toggle_hidden, Action::ToggleHidden),
    ] {
        if text.trim().is_empty() {
            continue;
        }
        match Shortcut::from_str(text) {
            Ok(s) => match gs.register(s) {
                Ok(()) => bound.push((s, action)),
                Err(e) => errors.push(format!("{text}: {e}")),
            },
            Err(e) => errors.push(format!("{text}: {e}")),
        }
    }
    if !errors.is_empty() {
        eprintln!("[atajos] errores: {errors:?}");
    }
    *app.state::<Shared>().shortcuts.lock().unwrap() = bound;
    errors
}
