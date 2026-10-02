use dynamic_island_lib::settings::*;

#[test]
fn ajustes_parciales_rellenan_valores_por_defecto() {
    let s: Settings = serde_json::from_str(r#"{"appearance":{"scale":1.2}}"#).unwrap();
    assert_eq!(s.appearance.scale, 1.2);
    assert_eq!(s.appearance.background, "#000000");
    assert_eq!(s.shortcuts.toggle_expand, "Ctrl+Alt+Space");
    assert!(s.modules.media);
}

#[test]
fn dock_serializa_en_minusculas() {
    let d = Dock { edge: Edge::Left, offset: 0.3 };
    assert_eq!(serde_json::to_string(&d).unwrap(), r#"{"edge":"left","offset":0.3}"#);
}
