//! Notas rápidas persistentes en `%APPDATA%/com.dallamond.dynamicisland/notes.json`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub done: bool,
    pub created_ms: i64,
}

pub fn path(dir: PathBuf) -> PathBuf {
    dir.join("notes.json")
}

pub fn load(file: &PathBuf) -> Vec<Note> {
    std::fs::read_to_string(file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(file: &PathBuf, notes: &[Note]) -> std::io::Result<()> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(notes)?)?;
    std::fs::rename(tmp, file)
}
