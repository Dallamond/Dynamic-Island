//! Ventana de la isla: estilos Win32, colocación por monitor y estado geométrico compartido.
//!
//! La ventana es un lienzo transparente de tamaño fijo que ignora el ratón. Solo la píldora
//! (cuyo rectángulo envía el frontend) recibe eventos: lo decide el hilo de `cursor`.
//! Nunca se usa hide()/show() salvo el primer show: para ocultar se mueve fuera de pantalla.

pub mod cursor;
pub mod placement;

use crate::settings::{Dock, Edge};
use crate::Shared;
use placement::Rect;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Tamaño lógico del lienzo (a escala 1.0). Cabe la isla expandida más su sombra.
pub const CANVAS_W: f64 = 480.0;
pub const CANVAS_H: f64 = 300.0;
/// Posición usada para "ocultar" la ventana sin hide().
const OFFSCREEN: i32 = -32000;

/// Geometría cacheada para que el hilo del cursor no consulte a la ventana cada 50 ms.
#[derive(Clone, Debug)]
pub struct Geo {
    pub win_x: i32,
    pub win_y: i32,
    pub scale: f64,
    /// Rectángulo de la píldora en px lógicos relativos al lienzo (x, y, w, h).
    pub hit: (f64, f64, f64, f64),
    pub monitor: String,
    pub edge: Edge,
}

impl Default for Geo {
    fn default() -> Self {
        Self { win_x: OFFSCREEN, win_y: OFFSCREEN, scale: 1.0, hit: (0.0, 0.0, 0.0, 0.0), monitor: String::new(), edge: Edge::Top }
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

pub fn monitor_name(m: &Monitor) -> String {
    m.name().cloned().unwrap_or_else(|| format!("{}x{}@{},{}", m.size().width, m.size().height, m.position().x, m.position().y))
}

pub fn monitor_rect(m: &Monitor) -> Rect {
    Rect { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32 }
}

pub fn list_monitors(win: &WebviewWindow) -> Vec<MonitorInfo> {
    let primary = win.primary_monitor().ok().flatten().map(|m| monitor_name(&m));
    win.available_monitors()
        .unwrap_or_default()
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

/// Monitor activo según ajustes; si ya no existe, el principal.
fn active_monitor(win: &WebviewWindow, wanted: Option<&str>) -> Option<Monitor> {
    let mons = win.available_monitors().ok()?;
    wanted
        .and_then(|n| mons.iter().find(|m| monitor_name(m) == n).cloned())
        .or_else(|| win.primary_monitor().ok().flatten())
        .or_else(|| mons.into_iter().next())
}

/// Coloca la ventana según los ajustes. Con `animate`, desliza desde la posición actual.
pub fn place(app: &AppHandle, animate: bool) {
    let Some(win) = app.get_webview_window("main") else { return };
    let st = app.state::<Shared>();
    if st.hidden.load(Ordering::Relaxed) {
        move_window(&win, &st, OFFSCREEN, OFFSCREEN);
        return;
    }
    let (wanted, scale_setting, margin, positions) = {
        let s = st.settings.lock().unwrap();
        (s.active_monitor.clone(), s.appearance.scale, s.appearance.margin, s.positions.clone())
    };
    let Some(mon) = active_monitor(&win, wanted.as_deref()) else { return };
    let name = monitor_name(&mon);
    let dock: Dock = positions.get(&name).copied().unwrap_or_default();
    let mscale = mon.scale_factor();
    let w = (CANVAS_W * scale_setting * mscale).round() as i32;
    let h = (CANVAS_H * scale_setting * mscale).round() as i32;
    let (tx, ty) = placement::window_origin(monitor_rect(&mon), dock, w, h, (margin * mscale).round() as i32);

    // Primero mover (si cambia de monitor, Windows reajusta el DPI), luego fijar tamaño.
    let from = win.outer_position().map(|p| (p.x, p.y)).unwrap_or((tx, ty));
    if animate && from.0 > OFFSCREEN / 2 {
        const STEPS: i32 = 10;
        for i in 1..STEPS {
            // ease-out cúbico
            let t = i as f64 / STEPS as f64;
            let e = 1.0 - (1.0 - t).powi(3);
            let x = from.0 + ((tx - from.0) as f64 * e) as i32;
            let y = from.1 + ((ty - from.1) as f64 * e) as i32;
            move_window(&win, &st, x, y);
            std::thread::sleep(std::time::Duration::from_millis(14));
        }
    }
    move_window(&win, &st, tx, ty);
    #[cfg(debug_assertions)]
    eprintln!("[place] monitor={name} rect={:?} escala={mscale} dock={dock:?} destino=({tx},{ty}) tam={w}x{h}", monitor_rect(&mon));
    if win.inner_size().map(|s| (s.width as i32, s.height as i32)).ok() != Some((w, h)) {
        let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
        move_window(&win, &st, tx, ty);
    }

    {
        let mut g = st.geo.lock().unwrap();
        g.scale = mscale;
        g.monitor = name.clone();
        g.edge = dock.edge;
    }
    let _ = app.emit("island://layout", Layout { edge: dock.edge, monitor: name });
}

pub fn move_window(win: &WebviewWindow, st: &Shared, x: i32, y: i32) {
    let _ = win.set_position(PhysicalPosition::new(x, y));
    let mut g = st.geo.lock().unwrap();
    g.win_x = x;
    g.win_y = y;
}

pub fn current_layout(st: &Shared) -> Layout {
    let g = st.geo.lock().unwrap();
    Layout { edge: g.edge, monitor: g.monitor.clone() }
}

/// Pasa la isla al siguiente monitor (orden por posición horizontal).
pub fn next_monitor(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    let mut mons = list_monitors(&win);
    mons.sort_by_key(|m| m.x);
    let st = app.state::<Shared>();
    let current = st.geo.lock().unwrap().monitor.clone();
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
