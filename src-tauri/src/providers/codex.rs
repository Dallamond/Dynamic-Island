//! Estado de Codex (app de escritorio y CLI) leyendo sus sesiones en `~/.codex/sessions/AAAA/MM/DD/rollout-*.jsonl`.
//!
//! Codex escribe una línea JSON por evento: inicio y fin de turno, llamadas a herramientas, tokens y límites.
//! Un hilo mira cada segundo si el archivo de la sesión más reciente ha crecido y lee solo las líneas nuevas.
//! No lee `auth.json` ni toca la configuración de Codex. Los permisos que pide Codex no quedan en el archivo,
//! así que el estado "esperando permiso" no existe para Codex.

use super::agent::{self, file_name, now_ms, truncate, AgentSession, AgentState, Limit, Status};
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager};

pub const CODEX: &str = "Codex";
pub const KIND: &str = "codex";
/// Cada cuántos segundos se busca un archivo de sesión más nuevo.
const RESCAN_EVERY: u32 = 5;
/// Una sesión sin actividad durante este tiempo no se muestra al arrancar ni se conserva.
const STALE_MS: i64 = 30 * 60_000;
/// Un turno "trabajando" sin eventos durante este tiempo se da por muerto (Codex cerrado a mitad).
const HUNG_MS: i64 = 10 * 60_000;
const MAX_FILES: usize = 8;

static RUNNING: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let mut running = RUNNING.lock().unwrap();
    match (enabled, running.is_some()) {
        (true, false) => {
            let Some(root) = sessions_dir() else { return };
            let flag = Arc::new(AtomicBool::new(true));
            let (app2, flag2) = (app.clone(), flag.clone());
            std::thread::Builder::new()
                .name("codex-watch".into())
                .spawn(move || watch(app2, root, flag2))
                .ok();
            *running = Some(flag);
        }
        (false, true) => {
            if let Some(flag) = running.take() {
                flag.store(false, Ordering::Relaxed);
            }
            app.state::<AgentState>().sessions.lock().unwrap().retain(|_, s| s.kind != KIND);
            agent::emit(app);
        }
        _ => {}
    }
}

fn sessions_dir() -> Option<PathBuf> {
    let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".codex")))?;
    Some(home.join("sessions"))
}

/// Archivo que se está siguiendo: posición leída y trozo de línea incompleta.
struct Tail {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
    session: Option<String>,
}

fn watch(app: AppHandle, root: PathBuf, flag: Arc<AtomicBool>) {
    let mut tail: Option<Tail> = None;
    let mut tick = 0u32;
    while flag.load(Ordering::Relaxed) {
        if tick % RESCAN_EVERY == 0 {
            if let Some(newest) = newest_rollout(&root) {
                if tail.as_ref().map(|t| &t.path) != Some(&newest) && modified_ago_ms(&newest) < STALE_MS {
                    tail = Some(Tail { path: newest, offset: 0, partial: Vec::new(), session: None });
                }
            }
        }
        tick = tick.wrapping_add(1);
        if let Some(t) = tail.as_mut() {
            let first_read = t.offset == 0;
            let lines = read_new_lines(t);
            if !lines.is_empty() {
                let changed = {
                    let st = app.state::<AgentState>();
                    let mut map = st.sessions.lock().unwrap();
                    let mut changed = false;
                    for v in &lines {
                        if let Some(id) = session_id_of(v) {
                            t.session = Some(id);
                        }
                        let Some(id) = t.session.clone() else { continue };
                        let s = map.entry(id.clone()).or_insert_with(|| AgentSession { id, kind: KIND.into(), agent: CODEX.into(), ..Default::default() });
                        changed |= apply_line(s, v);
                    }
                    if first_read {
                        // Al engancharse a una sesión ya empezada, un turno sin cerrar y viejo no es "trabajando".
                        if let Some(s) = t.session.as_ref().and_then(|id| map.get_mut(id)) {
                            if s.status == Status::Working && now_ms() - s.last_event_ms > HUNG_MS {
                                s.status = Status::Idle;
                            }
                        }
                    }
                    let now = now_ms();
                    map.retain(|_, s| s.kind != KIND || now - s.last_event_ms < STALE_MS);
                    changed
                };
                if changed {
                    agent::emit(&app);
                }
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn modified_ago_ms(p: &Path) -> i64 {
    p.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(i64::MAX)
}

/// Subcarpetas ordenadas por nombre, de la última a la primera.
fn subdirs_desc(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(p).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v.reverse();
    v
}

/// El `rollout-*.jsonl` modificado más recientemente entre los dos últimos días con sesiones.
fn newest_rollout(root: &Path) -> Option<PathBuf> {
    let mut days = Vec::new();
    'outer: for y in subdirs_desc(root).into_iter().take(2) {
        for m in subdirs_desc(&y).into_iter().take(2) {
            for d in subdirs_desc(&m) {
                days.push(d);
                if days.len() == 2 {
                    break 'outer;
                }
            }
        }
    }
    days.iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|p| p.metadata().and_then(|m| m.modified()).ok().map(|t| (t, p)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

fn read_new_lines(t: &mut Tail) -> Vec<Value> {
    let Ok(mut f) = File::open(&t.path) else { return Vec::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    if len < t.offset {
        // El archivo se ha truncado o reescrito: empezar de nuevo.
        t.offset = 0;
        t.partial.clear();
    }
    if len == t.offset || f.seek(SeekFrom::Start(t.offset)).is_err() {
        return Vec::new();
    }
    let mut buf = Vec::new();
    if f.take(len - t.offset).read_to_end(&mut buf).is_err() {
        return Vec::new();
    }
    t.offset += buf.len() as u64;
    t.partial.extend_from_slice(&buf);
    let Some(last_nl) = t.partial.iter().rposition(|&b| b == b'\n') else { return Vec::new() };
    let complete: Vec<u8> = t.partial.drain(..=last_nl).collect();
    complete.split(|&b| b == b'\n').filter_map(|l| serde_json::from_slice(l).ok()).collect()
}

fn session_id_of(v: &Value) -> Option<String> {
    if v.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    v.pointer("/payload/id").and_then(|x| x.as_str()).map(String::from)
}

/// "2026-10-01T11:50:00.106Z" → epoch en ms (UTC). Sin dependencias de fechas.
pub fn parse_ts(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<i64>());
    let (y, m, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
    let time = time.trim_end_matches('Z');
    let (hms, frac) = time.split_once('.').unwrap_or((time, "0"));
    let mut t = hms.split(':').map(|x| x.parse::<i64>());
    let (h, mi, sec) = (t.next()?.ok()?, t.next()?.ok()?, t.next()?.ok()?);
    let ms: i64 = format!("{:0<3}", &frac[..frac.len().min(3)]).parse().ok()?;
    // Días desde 1970-01-01 (algoritmo days_from_civil de Howard Hinnant).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + mi) * 60_000 + sec * 1000 + ms)
}

fn s<'a>(v: &'a Value, p: &str) -> Option<&'a str> {
    v.pointer(p).and_then(|x| x.as_str())
}

/// Nombre de una ventana de límite según sus minutos: (largo, corto).
fn window_label(minutes: u64) -> (String, String) {
    match minutes {
        300 => ("5 horas".into(), "5h".into()),
        10_080 => ("Semana".into(), "Sem".into()),
        43_200 => ("Mes".into(), "Mes".into()),
        m if m < 1440 => (format!("{} h", m / 60), format!("{}h", m / 60)),
        m => (format!("{} días", m / 1440), format!("{}d", m / 1440)),
    }
}

/// Archivos que toca un `apply_patch` ("*** Update File: ruta"), también dentro del código de `exec`.
fn patch_files(input: &str) -> Vec<String> {
    input
        .replace("\\n", "\n")
        .lines()
        .filter_map(|l| l.strip_prefix("*** Update File: ").or(l.strip_prefix("*** Add File: ")).or(l.strip_prefix("*** Delete File: ")))
        .map(|p| file_name(p.trim()))
        .collect()
}

fn describe_call(name: &str, args: &Value) -> String {
    let cmd = args.get("command").and_then(|c| c.as_str().map(String::from).or_else(|| c.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" "))));
    match (name, cmd) {
        ("shell_command" | "shell" | "exec_command" | "local_shell", Some(c)) => format!("Shell {}", truncate(&c, 48)),
        _ => name.to_string(),
    }
}

/// Valor de texto de `clave:"..."` o `"clave":"..."` dentro de código JS.
fn js_string_arg(code: &str, key: &str) -> Option<String> {
    let start = [format!("{key}:\""), format!("\"{key}\":\"")].iter().find_map(|k| code.find(k.as_str()).map(|i| i + k.len()))?;
    let mut out = String::new();
    let mut chars = code[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// Resumen de una llamada `exec` (Codex en modo código: `await tools.exec_command({cmd: ...})`).
pub fn describe_exec(code: &str) -> String {
    let tool = code.split("tools.").nth(1).and_then(|t| t.split('(').next()).unwrap_or("exec");
    match tool {
        "exec_command" | "shell_command" | "shell" => format!("Shell {}", truncate(&js_string_arg(code, "cmd").or_else(|| js_string_arg(code, "command")).unwrap_or_default(), 48)),
        t if t.starts_with("web__") => match js_string_arg(code, "q") {
            Some(q) => format!("Buscar {}", truncate(&q, 44)),
            None => "Web".into(),
        },
        t => t.replace("__", " "),
    }
}

/// Aplica una línea del rollout a la sesión. Devuelve si cambió algo visible.
pub fn apply_line(sess: &mut AgentSession, v: &Value) -> bool {
    let ts = s(v, "/timestamp").and_then(parse_ts).unwrap_or_else(now_ms);
    let p = v.get("payload").cloned().unwrap_or(Value::Null);
    let kind = s(v, "/type").unwrap_or("");
    let sub = s(&p, "/type").unwrap_or("");
    let set_cwd = |sess: &mut AgentSession, cwd: Option<&str>| {
        if let Some(cwd) = cwd {
            sess.cwd = cwd.to_string();
            sess.project = file_name(cwd.trim_end_matches(['/', '\\']));
        }
    };
    let changed = match (kind, sub) {
        ("session_meta", _) => {
            set_cwd(sess, s(&p, "/cwd"));
            true
        }
        ("turn_context", _) => {
            set_cwd(sess, s(&p, "/cwd"));
            if let Some(m) = s(&p, "/model") {
                sess.usage.model = Some(m.to_string());
            }
            true
        }
        ("event_msg", "task_started") => {
            sess.status = Status::Working;
            sess.turn_started_ms = Some(p.get("started_at").and_then(|x| x.as_i64()).map(|x| x * 1000).unwrap_or(ts));
            sess.turn_ended_ms = None;
            sess.files.clear();
            sess.tool_count = 0;
            sess.last_action = None;
            sess.message = None;
            if let Some(w) = p.get("model_context_window").and_then(|x| x.as_u64()) {
                sess.usage.context_size = Some(w);
            }
            true
        }
        ("event_msg", "item_completed") if s(&p, "/item/type") == Some("UserMessage") => {
            let text = p.pointer("/item/content").and_then(|c| c.as_array()).and_then(|a| a.iter().find_map(|x| x.get("text")?.as_str()));
            sess.message = text.map(|t| truncate(t, 120));
            true
        }
        ("response_item", "function_call") => {
            let name = s(&p, "/name").unwrap_or("herramienta");
            let args: Value = s(&p, "/arguments").and_then(|a| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
            sess.last_action = Some(describe_call(name, &args));
            sess.tool_count += 1;
            true
        }
        ("response_item", "custom_tool_call") => {
            let name = s(&p, "/name").unwrap_or("herramienta");
            let input = s(&p, "/input").unwrap_or("");
            let files = patch_files(input);
            sess.last_action = Some(match files.first() {
                Some(f) => format!("Edit {f}"),
                None if name == "exec" => describe_exec(input),
                None => name.to_string(),
            });
            for f in files.into_iter().rev() {
                sess.files.retain(|x| x != &f);
                sess.files.insert(0, f);
            }
            sess.files.truncate(MAX_FILES);
            sess.tool_count += 1;
            true
        }
        ("response_item", "web_search_call") => {
            sess.last_action = Some(format!("Buscar {}", truncate(s(&p, "/action/query").unwrap_or(""), 44)));
            sess.tool_count += 1;
            true
        }
        ("event_msg", "token_count") => {
            let u = |q: &str| p.pointer(q).and_then(|x| x.as_u64());
            let window = u("/info/model_context_window").or(sess.usage.context_size);
            let used = u("/info/last_token_usage/total_tokens");
            sess.usage.context_size = window;
            sess.usage.input_tokens = used;
            sess.usage.output_tokens = u("/info/total_token_usage/output_tokens");
            sess.usage.context_pct = match (used, window) {
                (Some(a), Some(b)) if b > 0 => Some(a as f64 * 100.0 / b as f64),
                _ => sess.usage.context_pct,
            };
            let limits: Vec<Limit> = ["primary", "secondary"]
                .iter()
                .filter_map(|k| p.pointer(&format!("/rate_limits/{k}")).filter(|x| x.is_object()))
                .map(|l| {
                    let (label, short) = window_label(l.get("window_minutes").and_then(|x| x.as_u64()).unwrap_or(0));
                    Limit { label, short, pct: l.get("used_percent").and_then(|x| x.as_f64()), resets: l.get("resets_at").and_then(|x| x.as_i64()) }
                })
                .collect();
            if !limits.is_empty() {
                sess.usage.limits = limits;
            }
            true
        }
        ("event_msg", "task_complete") => {
            sess.status = Status::Done;
            sess.turn_ended_ms = Some(p.get("completed_at").and_then(|x| x.as_i64()).map(|x| x * 1000).unwrap_or(ts));
            if let Some(m) = s(&p, "/last_agent_message") {
                sess.message = Some(truncate(m, 160));
            }
            true
        }
        ("event_msg", "turn_aborted") => {
            sess.status = Status::Done;
            sess.turn_ended_ms = Some(ts);
            sess.message = Some("Turno interrumpido".into());
            true
        }
        _ => false,
    };
    if changed {
        sess.last_event_ms = sess.last_event_ms.max(ts);
    }
    changed
}
