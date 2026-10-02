//! Un único hilo que consulta el cursor para todas las islas: hover (50 ms), arrastre propio
//! (~120 Hz) y, cada 500 ms, pantalla completa / monitor activo para el estado mínimo.
//!
//! - Hover: si el cursor entra en la píldora de una isla, esa ventana deja de ignorar el ratón
//!   y se avisa a su frontend; al salir, vuelve a ser click-through.
//! - Arrastre: el frontend llama a `drag_begin` al pulsar; mientras el botón izquierdo siga
//!   pulsado, este hilo mueve la ventana. Al soltar se imanta al borde más cercano.

use super::{fullscreen, island_hwnds, monitor_name, monitor_rect, placement, Geo};
use crate::Shared;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

const HOVER_POLL: Duration = Duration::from_millis(50);
/// Con el cursor lejos de todas las islas se consulta con menos frecuencia.
const IDLE_POLL: Duration = Duration::from_millis(120);
const DRAG_POLL: Duration = Duration::from_millis(8);
/// Píxeles de movimiento para que un clic pase a ser arrastre.
const DRAG_THRESHOLD: i32 = 6;
const MONITOR_CHECK: Duration = Duration::from_secs(3);
const FOREGROUND_CHECK: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub struct Drag {
    pub label: String,
    pub cursor0: (i32, i32),
    pub win0: (i32, i32),
    pub moved: bool,
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

fn lbutton_down() -> bool {
    unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 }
}

fn near_window(g: &Geo, px: i32, py: i32) -> bool {
    let w = (super::CANVAS_W * g.scale) as i32;
    let h = (super::CANVAS_H * g.scale) as i32;
    let pad = 200;
    px > g.win_x - pad && px < g.win_x + w + pad && py > g.win_y - pad && py < g.win_y + h + pad
}

fn monitors_signature(app: &AppHandle) -> String {
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| format!("{}:{:?}:{}", monitor_name(m), monitor_rect(m), m.scale_factor()))
        .collect::<Vec<_>>()
        .join("|")
}

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("cursor".into())
        .spawn(move || run(app))
        .expect("no se pudo crear el hilo del cursor");
}

fn run(app: AppHandle) {
    let st = app.state::<Shared>();
    let mut hovered: HashSet<String> = HashSet::new();
    let mut minimal: HashMap<String, bool> = HashMap::new();
    let mut last_mon_check = Instant::now();
    let mut last_fg_check = Instant::now() - FOREGROUND_CHECK;
    let mut fg_state = fullscreen::Foreground::default();
    let mut mon_sig = monitors_signature(&app);

    loop {
        let (px, py) = cursor_pos();

        // --- Arrastre ---
        let drag = st.drag.lock().unwrap().clone();
        if let Some(mut d) = drag {
            let Some(win) = app.get_webview_window(&d.label) else {
                *st.drag.lock().unwrap() = None;
                continue;
            };
            if !lbutton_down() {
                *st.drag.lock().unwrap() = None;
                if d.moved {
                    finish_drag(&app, &d.label, px, py);
                    let _ = app.emit_to(d.label.as_str(), "island://drag", false);
                }
                continue;
            }
            let (dx, dy) = (px - d.cursor0.0, py - d.cursor0.1);
            if !d.moved && dx.abs() + dy.abs() > DRAG_THRESHOLD {
                d.moved = true;
                *st.drag.lock().unwrap() = Some(d.clone());
                let _ = app.emit_to(d.label.as_str(), "island://drag", true);
            }
            if d.moved {
                super::move_window(&win, &st, d.win0.0 + dx, d.win0.1 + dy);
            }
            std::thread::sleep(DRAG_POLL);
            continue;
        }

        // --- Hover de cada isla ---
        let geos: Vec<(String, Geo)> = st.geos.lock().unwrap().iter().map(|(l, g)| (l.clone(), g.clone())).collect();
        let hidden = st.hidden.load(Ordering::Relaxed);
        let mut near = false;
        for (label, g) in &geos {
            let inside = !hidden && g.hit_contains(px, py, 2);
            near |= inside || near_window(g, px, py);
            if inside != hovered.contains(label) {
                if inside {
                    hovered.insert(label.clone());
                } else {
                    hovered.remove(label);
                }
                if let Some(w) = app.get_webview_window(label) {
                    let _ = w.set_ignore_cursor_events(!inside);
                }
                let _ = app.emit_to(label.as_str(), "island://hover", inside);
            }
        }

        // --- Pantalla completa / monitor activo → estado mínimo ---
        if last_fg_check.elapsed() >= FOREGROUND_CHECK {
            last_fg_check = Instant::now();
            let rects: Vec<placement::Rect> = geos.iter().map(|(_, g)| g.monitor_rect).collect();
            match fullscreen::query(&rects, &island_hwnds(&app)) {
                Some(fg) => fg_state = fg,
                // La activa es una isla (p. ej. al escribir una nota): no hay pantalla completa,
                // y el monitor activo se conserva.
                None => fg_state.fullscreen = None,
            }
            let (in_fs, on_active, multi) = {
                let s = st.settings.lock().unwrap();
                (s.minimal_in_fullscreen, s.minimal_on_active_monitor, s.multi_monitor)
            };
            for (i, (label, _)) in geos.iter().enumerate() {
                let m = (in_fs && fg_state.fullscreen == Some(i)) || (on_active && multi && fg_state.active == Some(i));
                if minimal.get(label) != Some(&m) {
                    minimal.insert(label.clone(), m);
                    let _ = app.emit_to(label.as_str(), "island://minimal", m);
                }
            }
        }

        // --- Cambios de monitores (conectar/desconectar, resolución, escala) ---
        if last_mon_check.elapsed() >= MONITOR_CHECK {
            last_mon_check = Instant::now();
            let sig = monitors_signature(&app);
            if sig != mon_sig {
                mon_sig = sig;
                super::place(&app, false);
            }
        }

        std::thread::sleep(if near { HOVER_POLL } else { IDLE_POLL });
    }
}

/// Al soltar: se imanta al borde más cercano.
/// - Isla principal: puede cambiar de monitor (el monitor bajo el cursor pasa a ser el activo).
/// - Isla secundaria (multimonitor): se queda en su monitor.
fn finish_drag(app: &AppHandle, label: &str, px: i32, py: i32) {
    let st = app.state::<Shared>();
    let own = st.geos.lock().unwrap().get(label).map(|g| (g.monitor.clone(), g.monitor_rect));
    let mons: Vec<_> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (monitor_rect(m), monitor_name(m)))
        .collect();
    let (rect, name) = match (label, own) {
        ("main", _) | (_, None) => match placement::monitor_at(&mons, px, py) {
            Some((r, n)) => (*r, n.clone()),
            None => return,
        },
        (_, Some((n, r))) => (r, n),
    };
    let cx = px.clamp(rect.x, rect.x + rect.w - 1);
    let cy = py.clamp(rect.y, rect.y + rect.h - 1);
    let dock = placement::dock_from_point(rect, cx, cy);
    let is_main = label == "main";
    crate::update_settings_with(app, true, move |s| {
        if is_main {
            s.active_monitor = Some(name.clone());
        }
        s.positions.insert(name, dock);
    });
}
