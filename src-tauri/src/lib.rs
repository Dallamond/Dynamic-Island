//! Núcleo de la Dynamic Island: estado compartido, comandos y arranque.

pub mod providers;
pub mod settings;
mod shortcuts;
mod tray;
pub mod window;

use settings::Settings;
use std::collections::HashMap;
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
    /// Geometría de cada isla, por etiqueta de ventana.
    pub geos: Mutex<HashMap<String, Geo>>,
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
    providers::apply(app, next);
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
        .additional_browser_args(window::WEBVIEW_ARGS)
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
fn get_layout(window: tauri::WebviewWindow, state: tauri::State<Shared>) -> Layout {
    window::layout_of(&state, window.label())
}

/// El frontend informa del rectángulo de la píldora (px lógicos dentro del lienzo).
#[tauri::command]
fn set_hit_rect(window: tauri::WebviewWindow, state: tauri::State<Shared>, x: f64, y: f64, w: f64, h: f64) {
    state.geos.lock().unwrap().entry(window.label().to_string()).or_default().hit = (x, y, w, h);
}

/// Botón izquierdo pulsado sobre la píldora: el hilo del cursor decide si es clic o arrastre.
#[tauri::command]
fn drag_begin(window: tauri::WebviewWindow, state: tauri::State<Shared>) {
    let label = window.label().to_string();
    let g = state.geos.lock().unwrap().get(&label).cloned().unwrap_or_default();
    *state.drag.lock().unwrap() = Some(Drag { label, cursor0: cursor_pos(), win0: (g.win_x, g.win_y), moved: false });
}

/// Activa el foco de teclado de la isla mientras se escribe (notas, calculadora).
#[tauri::command]
fn set_focusable(window: tauri::WebviewWindow, focusable: bool) {
    window::set_focusable(&window, focusable);
}

#[tauri::command]
async fn open_settings(app: AppHandle) {
    open_settings_window(&app);
}

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<MonitorInfo> {
    window::list_monitors(&app)
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


#[tauri::command]
fn debug_log(msg: String) {
    eprintln!("[web] {msg}");
}

#[tauri::command]
fn agent_state(app: AppHandle) -> Vec<providers::agent::AgentSession> {
    providers::agent::snapshot(&app)
}

// ---------------------------------------------------------------- música y audio

#[tauri::command]
fn media_control(app: AppHandle, action: String, position_ms: Option<i64>) {
    use providers::media::{send, Control, Msg};
    let c = match action.as_str() {
        "playPause" => Control::PlayPause,
        "next" => Control::Next,
        "prev" => Control::Prev,
        "seek" => Control::Seek(position_ms.unwrap_or(0)),
        _ => return,
    };
    send(&app, Msg::Control(c));
}

#[tauri::command]
fn media_refresh(app: AppHandle) {
    providers::media::send(&app, providers::media::Msg::Refresh);
}

#[tauri::command]
async fn audio_state() -> Result<providers::audio::AudioState, String> {
    providers::audio::state().map_err(|e| e.to_string())
}

#[tauri::command]
async fn audio_set_volume(volume: f32) -> Result<(), String> {
    providers::audio::set_volume(volume).map_err(|e| e.to_string())
}

#[tauri::command]
async fn audio_set_mute(muted: bool) -> Result<(), String> {
    providers::audio::set_mute(muted).map_err(|e| e.to_string())
}

#[tauri::command]
async fn audio_set_device(id: String) -> Result<(), String> {
    providers::audio::set_default_device(&id).map_err(|e| e.to_string())
}
// ---------------------------------------------------------------- sistema y utilidades

#[tauri::command]
fn system_watch(app: AppHandle, on: bool) {
    providers::system::watch(&app, on);
}

#[tauri::command]
fn timer_state(app: AppHandle) -> providers::timers::TimersState {
    app.state::<std::sync::Arc<providers::timers::Timers>>().state.lock().unwrap().clone()
}

#[tauri::command]
fn timer_action(app: AppHandle, action: providers::timers::TimerAction) -> providers::timers::TimersState {
    providers::timers::action(&app, action)
}

fn notes_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map(providers::notes::path).map_err(|e| e.to_string())
}

#[tauri::command]
fn notes_get(app: AppHandle) -> Result<Vec<providers::notes::Note>, String> {
    Ok(providers::notes::load(&notes_path(&app)?))
}

#[tauri::command]
fn notes_save(app: AppHandle, notes: Vec<providers::notes::Note>) -> Result<(), String> {
    providers::notes::save(&notes_path(&app)?, &notes).map_err(|e| e.to_string())
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
            settings::migrate_from_old_identifier(&dir);
            let path = settings::path(dir);
            let loaded = settings::load(&path);
            app.manage(Shared {
                settings: Mutex::new(loaded.clone()),
                settings_path: path,
                geos: Mutex::new(HashMap::new()),
                drag: Mutex::new(None),
                hidden: AtomicBool::new(false),
                shortcuts: Mutex::new(Vec::new()),
            });
            app.manage(providers::media::MediaHandle::default());
            app.manage(providers::agent::AgentState::default());
            app.manage(providers::system::SystemState::default());
            app.manage(std::sync::Arc::new(providers::timers::Timers::default()));
            providers::timers::spawn(handle.clone());

            let win = app.get_webview_window("main").expect("falta la ventana main");
            window::prepare(&win);
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
            quit,
            debug_log,
            agent_state,
            system_watch,
            timer_state,
            timer_action,
            notes_get,
            notes_save,
            media_control,
            media_refresh,
            audio_state,
            audio_set_volume,
            audio_set_mute,
            audio_set_device
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la Dynamic Island");
}
