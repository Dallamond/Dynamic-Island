//! ¿Hay algo a pantalla completa? ¿En qué monitor está la ventana activa?
//! Se consulta desde el hilo del cursor cada ~500 ms: dos llamadas Win32 baratas.

use super::placement::Rect;
use std::collections::HashMap;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect};

/// Ventanas del escritorio y la barra de tareas: ocupan todo el monitor pero no son "pantalla completa".
const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Foreground {
    /// Monitor (índice en la lista pasada) que tiene una ventana a pantalla completa.
    pub fullscreen: Option<usize>,
    /// Monitor donde está la ventana activa.
    pub active: Option<usize>,
}

/// Pura: ¿cubre `win` por completo a `mon`?
pub fn covers(win: Rect, mon: Rect) -> bool {
    win.x <= mon.x && win.y <= mon.y && win.x + win.w >= mon.x + mon.w && win.y + win.h >= mon.y + mon.h
}

/// Devuelve `None` si la ventana activa es una isla.
pub fn query(monitors: &[Rect], own: &HashMap<isize, String>) -> Option<Foreground> {
    unsafe {
        let fg: HWND = GetForegroundWindow();
        if fg.0.is_null() {
            return Some(Foreground::default());
        }
        if own.contains_key(&(fg.0 as isize)) {
            return None;
        }
        let mut buf = [0u16; 64];
        let n = GetClassNameW(fg, &mut buf) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return Some(Foreground::default());
        }
        let win = Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top };
        let (cx, cy) = (win.x + win.w / 2, win.y + win.h / 2);
        let active = monitors.iter().position(|m| m.contains(cx, cy));
        let shell = SHELL_CLASSES.contains(&class.as_str());
        let d3d = SHQueryUserNotificationState()
            .map(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
            .unwrap_or(false);
        let fullscreen = if shell {
            None
        } else if d3d {
            active
        } else {
            monitors.iter().position(|m| covers(win, *m))
        };
        Some(Foreground { fullscreen, active: if shell { None } else { active } })
    }
}
