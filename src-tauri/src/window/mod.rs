//! Ventanas de la isla: estilos Win32, colocación por monitor y estado geométrico compartido.
//!
//! Cada isla es un lienzo transparente de tamaño fijo que ignora el ratón. Solo la píldora
//! (cuyo rectángulo envía el frontend) recibe eventos: lo decide el hilo de `cursor`.
//! Nunca se usa hide()/show() salvo el primer show: para ocultar se mueve fuera de pantalla.
//!
//! Modo normal: una isla ("main") en el monitor activo.
//! Modo multimonitor: "main" en el monitor activo y una "island-N" en cada uno de los demás.

pub mod cursor;
pub mod fullscreen;
pub mod placement;

use crate::settings::{Dock, Edge};
use crate::Shared;
use placement::Rect;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Tamaño lógico del lienzo (a escala 1.0). Cabe la isla expandida más su sombra.
pub const CANVAS_W: f64 = 480.0;
pub const CANVAS_H: f64 = 300.0;
/// Posición usada para "ocultar" la ventana sin hide().
const OFFSCREEN: i32 = -32000;

/// Geometría cacheada de una isla, para que el hilo del cursor no consulte a la ventana.
#[derive(Clone, Debug)]
pub struct Geo {
    pub win_x: i32,
    pub win_y: i32,
    pub scale: f64,
    /// Rectángulo de la píldora en px lógicos relativos al lienzo (x, y, w, h).
    pub hit: (f64, f64, f64, f64),
    pub monitor: String,
    pub monitor_rect: Rect,
    pub edge: Edge,
}

impl Default for Geo {
    fn default() -> Self {
        Self {
            win_x: OFFSCREEN,
            win_y: OFFSCREEN,
            scale: 1.0,
            hit: (0.0, 0.0, 0.0, 0.0),
            monitor: String::new(),
            monitor_rect: Rect { x: 0, y: 0, w: 0, h: 0 },
            edge: Edge::Top,
        }
    }
}

impl Geo {
    /// ¿Está el punto físico (px, py) sobre la píldora? `pad` en px físicos.
    pub fn hit_contains(&self, px: i32, py: i32, pad: i32) -> bool {
        let (x, y, w, h) = self.hit;
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let r = Rect {
            x: self.win_x + (x * self.scale) as i32 - pad,
            y: self.win_y + (y * self.scale) as i32 - pad,
            w: (w * self.scale) as i32 + 2 * pad,
            h: (h * self.scale) as i32 + 2 * pad,
        };
        r.contains(px, py)
    }
}

#[derive(Serialize, Clone)]
pub struct MonitorInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

#[derive(Serialize, Clone)]
pub struct Layout {
    pub edge: Edge,
    pub monitor: String,
}

pub fn is_island(label: &str) -> bool {
    label == "main" || label.starts_with("island-")
}

pub fn island_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows().into_iter().filter(|(l, _)| is_island(l)).map(|(_, w)| w).collect()
}

pub fn monitor_name(m: &Monitor) -> String {
    m.name().cloned().unwrap_or_else(|| format!("{}x{}@{},{}", m.size().width, m.size().height, m.position().x, m.position().y))
}

pub fn monitor_rect(m: &Monitor) -> Rect {
    Rect { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32 }
}

fn all_monitors(app: &AppHandle) -> Vec<Monitor> {
    let mut mons = app.available_monitors().unwrap_or_default();
    mons.sort_by_key(|m| (m.position().x, m.position().y));
    mons
}

pub fn list_monitors(app: &AppHandle) -> Vec<MonitorInfo> {
    let primary = app.primary_monitor().ok().flatten().map(|m| monitor_name(&m));
    all_monitors(app)
        .iter()
        .map(|m| MonitorInfo {
            name: monitor_name(m),
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
            primary: primary.as_deref() == Some(&monitor_name(m)),
        })
        .collect()
}

/// Etiqueta estable de la isla secundaria de un monitor (`\\.\DISPLAY2` → `island-DISPLAY2`).
fn secondary_label(monitor: &str) -> String {
    let clean: String = monitor.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("island-{clean}")
}

/// Qué isla va en qué monitor según los ajustes.
fn assignments(app: &AppHandle) -> Vec<(String, Monitor)> {
    let st = app.state::<Shared>();
    let (wanted, multi) = {
        let s = st.settings.lock().unwrap();
        (s.active_monitor.clone(), s.multi_monitor)
    };
    let mons = all_monitors(app);
    let active = wanted
        .and_then(|n| mons.iter().find(|m| monitor_name(m) == n).cloned())
        .or_else(|| app.primary_monitor().ok().flatten())
        .or_else(|| mons.first().cloned());
    let Some(active) = active else { return Vec::new() };
    let active_name = monitor_name(&active);
    let mut out = vec![("main".to_string(), active)];
    if multi {
        for m in mons.into_iter().filter(|m| monitor_name(m) != active_name) {
            out.push((secondary_label(&monitor_name(&m)), m));
        }
    }
    out
}

/// Coloca todas las islas; crea las que falten y cierra las que sobren.
pub fn place(app: &AppHandle, animate: bool) {
    let plan = assignments(app);
    let wanted: Vec<&String> = plan.iter().map(|(l, _)| l).collect();
    for w in island_windows(app) {
        if w.label() != "main" && !wanted.contains(&&w.label().to_string()) {
            app.state::<Shared>().geos.lock().unwrap().remove(w.label());
            let _ = w.destroy();
        }
    }
    for (label, mon) in plan {
        match app.get_webview_window(&label) {
            Some(win) => place_one(app, &win, &mon, animate),
            None => create_island(app, label),
        }
    }
}

/// Crea una isla secundaria en el hilo principal (crear ventanas desde un comando síncrono
/// bloquearía el bucle de eventos en Windows).
fn create_island(app: &AppHandle, label: String) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if app2.get_webview_window(&label).is_some() {
            return;
        }
        let built = WebviewWindowBuilder::new(&app2, &label, WebviewUrl::App("index.html".into()))
            .title("Dynamic Island")
            .inner_size(CANVAS_W, CANVAS_H)
            .position(OFFSCREEN as f64, OFFSCREEN as f64)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(false)
            .visible(false)
            .build();
        match built {
            Ok(win) => {
                prepare(&win);
                // Reintenta colocar todo ya con la ventana creada.
                let app3 = app2.clone();
                std::thread::spawn(move || {
                    place(&app3, false);
                    if let Some(w) = app3.get_webview_window(win.label()) {
                        let _ = w.show();
                    }
                });
            }
            Err(e) => eprintln!("[isla] no se pudo crear {label}: {e}"),
        }
    });
}

/// Estilos y click-through iniciales de una isla recién creada.
pub fn prepare(win: &WebviewWindow) {
    apply_win32_styles(win);
    let _ = win.set_ignore_cursor_events(true);
}

fn place_one(app: &AppHandle, win: &WebviewWindow, mon: &Monitor, animate: bool) {
    let st = app.state::<Shared>();
    let label = win.label().to_string();
    if st.hidden.load(Ordering::Relaxed) {
        move_window(win, &st, OFFSCREEN, OFFSCREEN);
        return;
    }
    let (scale_setting, margin, positions) = {
        let s = st.settings.lock().unwrap();
        (s.appearance.scale, s.appearance.margin, s.positions.clone())
    };
    let name = monitor_name(mon);
    let dock: Dock = positions.get(&name).copied().unwrap_or_default();
    let mscale = mon.scale_factor();
    let w = (CANVAS_W * scale_setting * mscale).round() as i32;
    let h = (CANVAS_H * scale_setting * mscale).round() as i32;
    let rect = monitor_rect(mon);
    let (tx, ty) = placement::window_origin(rect, dock, w, h, (margin * mscale).round() as i32);

    // Primero mover (si cambia de monitor, Windows reajusta el DPI), luego fijar tamaño.
    let from = win.outer_position().map(|p| (p.x, p.y)).unwrap_or((tx, ty));
    if animate && from.0 > OFFSCREEN / 2 {
        const STEPS: i32 = 10;
        for i in 1..STEPS {
            let t = i as f64 / STEPS as f64;
            let e = 1.0 - (1.0 - t).powi(3); // ease-out cúbico
            let x = from.0 + ((tx - from.0) as f64 * e) as i32;
            let y = from.1 + ((ty - from.1) as f64 * e) as i32;
            move_window(win, &st, x, y);
            std::thread::sleep(std::time::Duration::from_millis(14));
        }
    }
    move_window(win, &st, tx, ty);
    if win.inner_size().map(|s| (s.width as i32, s.height as i32)).ok() != Some((w, h)) {
        let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
        move_window(win, &st, tx, ty);
    }

    {
        let mut geos = st.geos.lock().unwrap();
        let g = geos.entry(label.clone()).or_default();
        g.scale = mscale;
        g.monitor = name.clone();
        g.monitor_rect = rect;
        g.edge = dock.edge;
    }
    let _ = app.emit_to(label.as_str(), "island://layout", Layout { edge: dock.edge, monitor: name });
}

pub fn move_window(win: &WebviewWindow, st: &Shared, x: i32, y: i32) {
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let mut geos = st.geos.lock().unwrap();
    let g = geos.entry(win.label().to_string()).or_default();
    g.win_x = x;
    g.win_y = y;
}

pub fn layout_of(st: &Shared, label: &str) -> Layout {
    let geos = st.geos.lock().unwrap();
    let g = geos.get(label).cloned().unwrap_or_default();
    Layout { edge: g.edge, monitor: g.monitor }
}

/// Isla del monitor donde está el cursor (o la principal).
pub fn island_under_cursor(app: &AppHandle) -> String {
    let (px, py) = cursor::cursor_pos();
    let st = app.state::<Shared>();
    let geos = st.geos.lock().unwrap();
    geos.iter()
        .find(|(_, g)| g.monitor_rect.contains(px, py))
        .map(|(l, _)| l.clone())
        .unwrap_or_else(|| "main".into())
}

/// Pasa la isla principal al siguiente monitor (orden por posición horizontal).
pub fn next_monitor(app: &AppHandle) {
    let mons = list_monitors(app);
    let st = app.state::<Shared>();
    let current = st.geos.lock().unwrap().get("main").map(|g| g.monitor.clone()).unwrap_or_default();
    let idx = mons.iter().position(|m| m.name == current).map(|i| (i + 1) % mons.len()).unwrap_or(0);
    if let Some(m) = mons.get(idx) {
        crate::update_settings(app, |s| s.active_monitor = Some(m.name.clone()));
    }
}

pub fn toggle_hidden(app: &AppHandle) {
    let st = app.state::<Shared>();
    st.hidden.fetch_xor(true, Ordering::Relaxed);
    place(app, false);
}

/// Estilos Win32: fuera de Alt+Tab y de la barra de tareas, y sin robar el foco al hacer clic.
pub fn apply_win32_styles(win: &WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;
    let Ok(h) = win.hwnd() else { return };
    let hwnd = HWND(h.0 as _);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex = (ex | WS_EX_TOOLWINDOW.0 as isize | WS_EX_NOACTIVATE.0 as isize) & !(WS_EX_APPWINDOW.0 as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
    }
}

/// Permite o impide que la isla reciba el foco de teclado (notas, calculadora).
pub fn set_focusable(win: &WebviewWindow, focusable: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;
    let Ok(h) = win.hwnd() else { return };
    let hwnd = HWND(h.0 as _);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let ex = if focusable { ex & !(WS_EX_NOACTIVATE.0 as isize) } else { ex | WS_EX_NOACTIVATE.0 as isize };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
        if focusable {
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// HWND de todas las islas (para no confundirlas con la ventana en primer plano).
pub fn island_hwnds(app: &AppHandle) -> HashMap<isize, String> {
    island_windows(app)
        .into_iter()
        .filter_map(|w| w.hwnd().ok().map(|h| (h.0 as isize, w.label().to_string())))
        .collect()
}
