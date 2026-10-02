//! Un único hilo que consulta el cursor: hover de la píldora (50 ms) y arrastre propio (~120 Hz).
//!
//! - Hover: si el cursor entra en el rectángulo de la píldora, la ventana deja de ignorar el
//!   ratón y se avisa al frontend; al salir, vuelve a ser click-through.
//! - Arrastre: el frontend llama a `drag_begin` al pulsar; mientras el botón izquierdo siga
//!   pulsado, este hilo mueve la ventana. Al soltar se imanta al borde más cercano.

use super::{monitor_name, monitor_rect, place, placement, Geo};
use crate::Shared;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

const HOVER_POLL: Duration = Duration::from_millis(50);
/// Con el cursor lejos de la isla se consulta con menos frecuencia.
const IDLE_POLL: Duration = Duration::from_millis(120);
const DRAG_POLL: Duration = Duration::from_millis(8);
/// Píxeles de movimiento para que un clic pase a ser arrastre.
const DRAG_THRESHOLD: i32 = 6;
/// Cada cuánto se comprueba si cambió la configuración de monitores.
const MONITOR_CHECK: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug)]
pub struct Drag {
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

/// Distancia aproximada del cursor a la ventana, para decidir el ritmo de sondeo.
fn near_window(g: &Geo, px: i32, py: i32) -> bool {
    let w = (super::CANVAS_W * g.scale) as i32;
    let h = (super::CANVAS_H * g.scale) as i32;
    let pad = 200;
    px > g.win_x - pad && px < g.win_x + w + pad && py > g.win_y - pad && py < g.win_y + h + pad
}

fn monitors_signature(app: &AppHandle) -> String {
    let Some(win) = app.get_webview_window("main") else { return String::new() };
    win.available_monitors()
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
    let Some(win) = app.get_webview_window("main") else { return };
    let st = app.state::<Shared>();
    let mut hovered = false;
    let mut last_mon_check = Instant::now();
    let mut mon_sig = monitors_signature(&app);

    loop {
        let (px, py) = cursor_pos();

        // --- Arrastre ---
        let drag = *st.drag.lock().unwrap();
        if let Some(mut d) = drag {
            if !lbutton_down() {
                *st.drag.lock().unwrap() = None;
                if d.moved {
                    finish_drag(&app, px, py);
                    let _ = app.emit("island://drag", false);
                }
                continue;
            }
            let (dx, dy) = (px - d.cursor0.0, py - d.cursor0.1);
            if !d.moved && dx.abs() + dy.abs() > DRAG_THRESHOLD {
                d.moved = true;
                *st.drag.lock().unwrap() = Some(d);
                let _ = app.emit("island://drag", true);
            }
            if d.moved {
                super::move_window(&win, &st, d.win0.0 + dx, d.win0.1 + dy);
            }
            std::thread::sleep(DRAG_POLL);
            continue;
        }

        // --- Hover ---
        let geo = st.geo.lock().unwrap().clone();
        let hidden = st.hidden.load(Ordering::Relaxed);
        let inside = !hidden && geo.hit_contains(px, py, 2);
        if inside != hovered {
            hovered = inside;
            let _ = win.set_ignore_cursor_events(!inside);
            let _ = app.emit("island://hover", inside);
        }

        // --- Cambios de monitores (conectar/desconectar, resolución, escala) ---
        if last_mon_check.elapsed() >= MONITOR_CHECK {
            last_mon_check = Instant::now();
            let sig = monitors_signature(&app);
            if sig != mon_sig {
                mon_sig = sig;
                place(&app, false);
            }
        }

        let near = hovered || near_window(&geo, px, py);
        std::thread::sleep(if near { HOVER_POLL } else { IDLE_POLL });
    }
}

/// Al soltar: monitor bajo el cursor + borde más cercano → se guarda y se coloca animando.
fn finish_drag(app: &AppHandle, px: i32, py: i32) {
    let Some(win) = app.get_webview_window("main") else { return };
    let mons: Vec<_> = win
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| (monitor_rect(m), monitor_name(m)))
        .collect();
    let Some((rect, name)) = placement::monitor_at(&mons, px, py) else { return };
    let dock = placement::dock_from_point(*rect, px, py);
    let name = name.clone();
    crate::update_settings_with(app, true, |s| {
        s.active_monitor = Some(name.clone());
        s.positions.insert(name, dock);
    });
}
