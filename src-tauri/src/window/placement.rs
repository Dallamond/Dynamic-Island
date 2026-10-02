//! Geometría pura del acoplado: sin Tauri ni Win32, para poder testearla.
//! Todo en píxeles físicos de escritorio virtual (pueden ser negativos con varios monitores).

use crate::settings::{Dock, Edge};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }

    /// Distancia al cuadrado desde un punto al rectángulo (0 si está dentro).
    pub fn dist2(&self, px: i32, py: i32) -> i64 {
        let dx = (self.x - px).max(0).max(px - (self.x + self.w - 1)) as i64;
        let dy = (self.y - py).max(0).max(py - (self.y + self.h - 1)) as i64;
        dx * dx + dy * dy
    }
}

/// Margen (fracción del borde) dentro del cual la isla se imanta al centro.
const CENTER_SNAP: f64 = 0.04;

/// Posición de la esquina superior izquierda de la ventana-lienzo para un acoplado.
/// - Arriba: la isla queda pegada arriba y centrada en `offset` horizontalmente.
/// - Laterales: la isla queda pegada al borde y centrada verticalmente en `offset`.
pub fn window_origin(mon: Rect, dock: Dock, win_w: i32, win_h: i32, margin: i32) -> (i32, i32) {
    let clamp_x = |x: i32| x.clamp(mon.x, (mon.x + mon.w - win_w).max(mon.x));
    let clamp_y = |y: i32| y.clamp(mon.y, (mon.y + mon.h - win_h).max(mon.y));
    match dock.edge {
        Edge::Top => {
            let cx = mon.x as f64 + dock.offset * mon.w as f64;
            (clamp_x((cx - win_w as f64 / 2.0).round() as i32), mon.y + margin)
        }
        Edge::Left | Edge::Right => {
            let cy = mon.y as f64 + dock.offset * mon.h as f64;
            let y = clamp_y((cy - win_h as f64 / 2.0).round() as i32);
            let x = if dock.edge == Edge::Left {
                mon.x + margin
            } else {
                mon.x + mon.w - margin - win_w
            };
            (x, y)
        }
    }
}

/// Decide a qué borde se imanta la isla al soltarla en (px, py) dentro de `mon`.
/// El borde inferior no se usa: la isla vive arriba o en un lateral.
pub fn dock_from_point(mon: Rect, px: i32, py: i32) -> Dock {
    let d_top = (py - mon.y).max(0);
    let d_left = (px - mon.x).max(0);
    let d_right = (mon.x + mon.w - px).max(0);
    let snap = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if (v - 0.5).abs() < CENTER_SNAP {
            0.5
        } else {
            v
        }
    };
    if d_top <= d_left && d_top <= d_right {
        Dock { edge: Edge::Top, offset: snap((px - mon.x) as f64 / mon.w as f64) }
    } else {
        let edge = if d_left <= d_right { Edge::Left } else { Edge::Right };
        Dock { edge, offset: snap((py - mon.y) as f64 / mon.h as f64) }
    }
}

/// Monitor que contiene el punto o, si cae entre monitores, el más cercano.
pub fn monitor_at<'a, T>(mons: &'a [(Rect, T)], px: i32, py: i32) -> Option<&'a (Rect, T)> {
    mons.iter()
        .find(|(r, _)| r.contains(px, py))
        .or_else(|| mons.iter().min_by_key(|(r, _)| r.dist2(px, py)))
}

