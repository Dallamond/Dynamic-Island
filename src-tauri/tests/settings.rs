use dynamic_island_lib::settings::*;

#[test]
fn ajustes_parciales_rellenan_valores_por_defecto() {
    let s: Settings = serde_json::from_str(r#"{"appearance":{"scale":1.2}}"#).unwrap();
    assert_eq!(s.appearance.scale, 1.2);
    assert_eq!(s.appearance.background, "#000000");
    assert_eq!(s.shortcuts.toggle_expand, "Ctrl+Alt+I");
    assert!(s.modules.media);
}

#[test]
fn dock_serializa_en_minusculas() {
    let d = Dock { edge: Edge::Left, offset: 0.3 };
    assert_eq!(serde_json::to_string(&d).unwrap(), r#"{"edge":"left","offset":0.3}"#);
}

#[test]
fn argumentos_webview_iguales_en_config_y_codigo() {
    // Si difieren, WebView2 se niega a crear la segunda ventana.
    let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let main = conf["app"]["windows"][0]["additionalBrowserArgs"].as_str().unwrap();
    assert_eq!(main, dynamic_island_lib::window::WEBVIEW_ARGS);
}
