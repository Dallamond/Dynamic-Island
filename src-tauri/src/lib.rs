//! Núcleo de la Dynamic Island: estado compartido, comandos y arranque.

pub mod settings;
mod shortcuts;
mod tray;
pub mod window;

use settings::Settings;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::Shortcut;
use window::cursor::{cursor_pos, Drag};
use window::{Geo, Layout, MonitorInfo};

/// Estado compartido entre comandos, hilos y proveedores.
pub struct Shared {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub geo: Mutex<Geo>,
    pub drag: Mutex<Option<Drag>>,
    pub hidden: AtomicBool,
    pub shortcuts: Mutex<Vec<(Shortcut, shortcuts::Action)>>,
}

/// Modifica los ajustes, los guarda y aplica lo que haya cambiado.
pub fn update_settings(app: &AppHandle, f: impl FnOnce(&mut Settings)) {
    update_settings_with(app, false, f)
}

pub fn update_settings_with(app: &AppHandle, animate: bool, f: impl FnOnce(&mut Settings)) {
    let st = app.state::<Shared>();
    let (prev, next) = {
        let mut s = st.settings.lock().unwrap();
        let prev = s.clone();
        f(&mut s);
        (prev, s.clone())
    };
    if let Err(e) = settings::save(&st.settings_path, &next) {
        eprintln!("[ajustes] no se pudieron guardar: {e}");
    }
    apply_settings(app, Some(&prev), &next, animate);
}

fn apply_settings(app: &AppHandle, prev: Option<&Settings>, next: &Settings, animate: bool) {
    let shortcuts_changed = prev.map_or(true, |p| {
        serde_json::to_value(&p.shortcuts).ok() != serde_json::to_value(&next.shortcuts).ok()
    });
    if shortcuts_changed {
        let errors = shortcuts::register(app, &next.shortcuts);
        if !errors.is_empty() {
            let _ = app.emit("settings://shortcut-errors", errors);
        }
    }
    if prev.map_or(true, |p| p.autostart != next.autostart) {
        apply_autostart(app, next.autostart);
    }
    window::place(app, animate);
    let _ = app.emit("settings://changed", next.clone());
}

fn apply_autostart(app: &AppHandle, enabled: bool) {
    // En desarrollo no se toca el registro: apuntaría al ejecutable de debug.
    if cfg!(debug_assertions) {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let current = al.is_enabled().unwrap_or(false);
    if current != enabled {
        let r = if enabled { al.enable() } else { al.disable() };
        if let Err(e) = r {
            eprintln!("[autostart] {e}");
        }
    }
}

pub fn open_settings_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let r = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Ajustes — Dynamic Island")
        .inner_size(600.0, 720.0)
        .min_inner_size(480.0, 480.0)
        .center()
        .build();
    if let Err(e) = r {
        eprintln!("[ajustes] no se pudo abrir la ventana: {e}");
    }
}

// ---------------------------------------------------------------- comandos

#[tauri::command]
fn get_settings(state: tauri::State<Shared>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) {
    update_settings(&app, move |s| *s = settings);
}

#[tauri::command]
fn get_layout(state: tauri::State<Shared>) -> Layout {
    window::current_layout(&state)
}

/// El frontend informa del rectángulo de la píldora (px lógicos dentro del lienzo).
#[tauri::command]
fn set_hit_rect(state: tauri::State<Shared>, x: f64, y: f64, w: f64, h: f64) {
    state.geo.lock().unwrap().hit = (x, y, w, h);
}

/// Botón izquierdo pulsado sobre la píldora: el hilo del cursor decide si es clic o arrastre.
#[tauri::command]
fn drag_begin(state: tauri::State<Shared>) {
    let g = state.geo.lock().unwrap().clone();
    *state.drag.lock().unwrap() = Some(Drag { cursor0: cursor_pos(), win0: (g.win_x, g.win_y), moved: false });
}

/// Activa el foco de teclado de la isla mientras se escribe (notas, calculadora).
#[tauri::command]
fn set_focusable(app: AppHandle, focusable: bool) {
    if let Some(w) = app.get_webview_window("main") {
        window::set_focusable(&w, focusable);
    }
}

#[tauri::command]
async fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    app.get_webview_window("main").map(|w| window::list_monitors(&w)).unwrap_or_default()
}

#[tauri::command]
fn move_to_monitor(app: AppHandle, name: String) {
    update_settings_with(&app, true, |s| s.active_monitor = Some(name));
}

#[tauri::command]
fn reset_position(app: AppHandle) {
    update_settings_with(&app, true, |s| {
        s.positions.clear();
        s.active_monitor = None;
    });
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

// ---------------------------------------------------------------- arranque

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Debe ir el primero: una segunda instancia solo abre los ajustes de la primera.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| open_settings_window(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(shortcuts::plugin())
        .setup(|app| {
            let handle = app.handle().clone();
            let dir = app.path().app_config_dir()?;
            let path = settings::path(dir);
            let loaded = settings::load(&path);
            app.manage(Shared {
                settings: Mutex::new(loaded.clone()),
                settings_path: path,
                geo: Mutex::new(Geo::default()),
                drag: Mutex::new(None),
                hidden: AtomicBool::new(false),
                shortcuts: Mutex::new(Vec::new()),
            });

            let win = app.get_webview_window("main").expect("falta la ventana main");
            window::apply_win32_styles(&win);
            let _ = win.set_ignore_cursor_events(true);
            apply_settings(&handle, None, &loaded, false);
            win.show()?; // único show(): a partir de aquí se mueve, nunca se oculta

            tray::create(&handle)?;
            window::cursor::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_layout,
            set_hit_rect,
            drag_begin,
            set_focusable,
            open_settings,
            list_monitors,
            move_to_monitor,
            reset_position,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la Dynamic Island");
}
