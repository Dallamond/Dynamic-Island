//! Proveedores de datos. Cada uno se enciende o apaga según los módulos activos en ajustes:
//! un módulo desactivado no consulta nada.

pub mod agent;
pub mod audio;
pub mod media;

use crate::settings::Settings;
use tauri::AppHandle;

pub fn apply(app: &AppHandle, s: &Settings) {
    media::set_enabled(app, s.modules.media);
    agent::set_enabled(app, s.modules.agent);
}
