use dynamic_island_lib::settings::{Dock, Edge};
use dynamic_island_lib::window::placement::*;

// Los tres monitores reales de Lucas.
const MAIN: Rect = Rect { x: 0, y: 0, w: 2560, h: 1440 };
const VERT: Rect = Rect { x: -1080, y: -294, w: 1080, h: 1920 };

#[test]
fn arriba_centrado() {
    let (x, y) = window_origin(MAIN, Dock::default(), 480, 300, 6);
    assert_eq!((x, y), (1040, 6));
}

#[test]
fn arriba_no_se_sale_del_monitor() {
    let d = Dock { edge: Edge::Top, offset: 0.0 };
    assert_eq!(window_origin(MAIN, d, 480, 300, 6).0, 0);
    let d = Dock { edge: Edge::Top, offset: 1.0 };
    assert_eq!(window_origin(MAIN, d, 480, 300, 6).0, 2560 - 480);
}

#[test]
fn laterales_con_coordenadas_negativas() {
    let d = Dock { edge: Edge::Left, offset: 0.5 };
    assert_eq!(window_origin(VERT, d, 480, 300, 6), (-1074, -294 + 960 - 150));
    let d = Dock { edge: Edge::Right, offset: 0.5 };
    assert_eq!(window_origin(VERT, d, 480, 300, 6).0, -6 - 480);
}

#[test]
fn soltar_cerca_del_borde_elige_ese_borde() {
    assert_eq!(dock_from_point(MAIN, 1280, 20).edge, Edge::Top);
    assert_eq!(dock_from_point(MAIN, 10, 700).edge, Edge::Left);
    assert_eq!(dock_from_point(MAIN, 2550, 700).edge, Edge::Right);
}

#[test]
fn imantado_al_centro() {
    assert_eq!(dock_from_point(MAIN, 1300, 10).offset, 0.5);
    assert!((dock_from_point(MAIN, 640, 10).offset - 0.25).abs() < 1e-9);
}

#[test]
fn monitor_por_punto() {
    let mons = [(MAIN, "1"), (VERT, "2")];
    assert_eq!(monitor_at(&mons, -500, 0).unwrap().1, "2");
    assert_eq!(monitor_at(&mons, 100, 100).unwrap().1, "1");
    // Hueco por encima del monitor principal: el más cercano.
    assert_eq!(monitor_at(&mons, 100, -100).unwrap().1, "1");
}
