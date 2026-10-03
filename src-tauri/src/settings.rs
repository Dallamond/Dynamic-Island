//! Ajustes persistentes en `%APPDATA%/com.dallamond.dynamicisland/settings.json`.
//! Rust es el dueño de los ajustes: el frontend los pide y los guarda por comandos.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Left,
    Right,
}

/// En qué monitores la píldora cerrada crece para dar más detalle de una actividad (Claude Code).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ActivityDetail {
    /// Solo en el monitor principal.
    #[default]
    Primary,
    All,
    Off,
}

/// Qué enseña con preferencia la isla de un monitor.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Focus {
    /// Lo más importante en cada momento.
    #[default]
    Auto,
    Agent,
    Timer,
    Media,
}

/// Dónde está acoplada la isla dentro de un monitor.
/// `offset` es la posición del centro de la isla a lo largo del borde (0..1).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Dock {
    pub edge: Edge,
    pub offset: f64,
}

impl Default for Dock {
    fn default() -> Self {
        Self { edge: Edge::Top, offset: 0.5 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub opacity: f64,
    /// Escala global de la isla (1.0 = tamaño normal).
    pub scale: f64,
    /// Separación en px lógicos entre la isla y el borde del monitor.
    pub margin: f64,
    pub show_seconds: bool,
    /// Tiempo con el ratón encima antes de expandir del todo.
    pub expand_delay_ms: u32,
    /// Tiempo de gracia al sacar el ratón antes de cerrar.
    pub collapse_delay_ms: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            background: "#000000".into(),
            foreground: "#ffffff".into(),
            accent: "#0a84ff".into(),
            opacity: 1.0,
            scale: 1.0,
            margin: 6.0,
            show_seconds: false,
            expand_delay_ms: 450,
            collapse_delay_ms: 350,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Modules {
    pub media: bool,
    pub agent: bool,
    /// Codex (app de escritorio o CLI), leído de sus archivos de sesión.
    pub codex: bool,
    /// IA local (Bionic / LM Studio u Ollama) por su API.
    pub local_ai: bool,
    pub system: bool,
    pub timer: bool,
    pub calc: bool,
    pub notes: bool,
}

impl Default for Modules {
    fn default() -> Self {
        Self { media: true, agent: true, codex: true, local_ai: true, system: true, timer: true, calc: true, notes: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct LocalAi {
    /// `http://host:puerto` del servidor (LM Studio/Bionic: 1234, Ollama: 11434).
    pub endpoint: String,
    /// Nombre en la isla; vacío = el del servidor detectado.
    pub name: String,
}

impl Default for LocalAi {
    fn default() -> Self {
        Self { endpoint: "http://127.0.0.1:1234".into(), name: String::new() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    pub toggle_expand: String,
    pub next_monitor: String,
    pub toggle_hidden: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            toggle_expand: "Ctrl+Alt+I".into(),
            next_monitor: "Ctrl+Alt+M".into(),
            toggle_hidden: "Ctrl+Alt+H".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub appearance: Appearance,
    pub modules: Modules,
    pub shortcuts: Shortcuts,
    pub local_ai: LocalAi,
    pub autostart: bool,
    /// Una isla en cada monitor a la vez.
    pub multi_monitor: bool,
    /// Estado mínimo en el monitor que tiene algo a pantalla completa.
    pub minimal_in_fullscreen: bool,
    /// Estado mínimo en el monitor donde está la ventana activa (solo multimonitor).
    pub minimal_on_active_monitor: bool,
    /// Píldora cerrada ampliada con el detalle de la actividad.
    pub activity_detail: ActivityDetail,
    /// Nombre del monitor activo (p. ej. `\\.\DISPLAY1`). `None` = principal.
    pub active_monitor: Option<String>,
    /// Posición guardada por monitor.
    pub positions: HashMap<String, Dock>,
    /// Módulo fijado por monitor (sin entrada = automático).
    pub monitor_focus: HashMap<String, Focus>,
}

/// Identificador anterior de la app (hasta la 0.2.0).
const OLD_IDENTIFIER: &str = "com.lucas.dynamicisland";

/// Si la carpeta de datos nueva no existe, copia ajustes y notas de la del identificador antiguo.
pub fn migrate_from_old_identifier(dir: &std::path::Path) {
    if dir.exists() {
        return;
    }
    let Some(old) = dir.parent().map(|p| p.join(OLD_IDENTIFIER)) else { return };
    if !old.exists() || std::fs::create_dir_all(dir).is_err() {
        return;
    }
    for f in ["settings.json", "notes.json"] {
        let _ = std::fs::copy(old.join(f), dir.join(f));
    }
}

pub fn path(dir: PathBuf) -> PathBuf {
    dir.join("settings.json")
}

pub fn load(file: &PathBuf) -> Settings {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Escritura atómica: fichero temporal + rename, para no corromper los ajustes si se corta.
pub fn save(file: &PathBuf, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(settings)?)?;
    std::fs::rename(tmp, file)
}


impl Default for Settings {
    fn default() -> Self {
        Self {
            appearance: Appearance::default(),
            modules: Modules::default(),
            shortcuts: Shortcuts::default(),
            local_ai: LocalAi::default(),
            autostart: false,
            multi_monitor: false,
            minimal_in_fullscreen: true,
            minimal_on_active_monitor: false,
            activity_detail: ActivityDetail::Primary,
            active_monitor: None,
            positions: HashMap::new(),
            monitor_focus: HashMap::new(),
        }
    }
}
