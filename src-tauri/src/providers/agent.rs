//! Estado de los agentes de código. Claude Code llega por HTTP local; Codex lo lee `codex.rs`.
//!
//! Claude Code: estado recibido por HTTP local (sin credenciales, sin sondeo).
//!
//! - `POST /hook`: hooks HTTP de Claude Code (SessionStart, UserPromptSubmit, PreToolUse...).
//! - `POST /status`: el JSON del statusLine (contexto, coste, límites de 5 h y semanal),
//!   reenviado por `scripts/island-statusline.mjs`.
//!
//! El servidor bloquea en `accept()`: con Claude Code cerrado no consume nada.
//! Si la isla está cerrada, Claude Code recibe "conexión rechazada" y sigue sin bloquearse.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

pub const PORT: u16 = 47823;
pub const CLAUDE: &str = "Claude Code";
/// Máximo de archivos tocados que se recuerdan por turno.
const MAX_FILES: usize = 8;

#[derive(Serialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Idle,
    Working,
    Waiting,
    Done,
    Error,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub model: Option<String>,
    pub context_pct: Option<f64>,
    pub context_size: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
    /// Límites de uso del plan (Claude: 5 h y semana; Codex: lo que diga su ventana).
    pub limits: Vec<Limit>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    /// Nombre largo para el panel ("5 horas").
    pub label: String,
    /// Nombre corto para la píldora ("5h").
    pub short: String,
    pub pct: Option<f64>,
    /// Epoch en segundos.
    pub resets: Option<i64>,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub agent: String,
    pub project: String,
    pub cwd: String,
    pub status: Status,
    /// Texto corto de la última acción ("Edit lib.rs", "Bash npm test").
    pub last_action: Option<String>,
    pub message: Option<String>,
    pub files: Vec<String>,
    pub tool_count: u32,
    pub turn_started_ms: Option<i64>,
    pub turn_ended_ms: Option<i64>,
    pub last_event_ms: i64,
    pub usage: Usage,
}

#[derive(Default)]
pub struct AgentState {
    pub(crate) sessions: Mutex<HashMap<String, AgentSession>>,
    running: Mutex<Option<Arc<AtomicBool>>>,
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let st = app.state::<AgentState>();
    let mut running = st.running.lock().unwrap();
    match (enabled, running.is_some()) {
        (true, false) => {
            let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[agente] no se pudo abrir el puerto {PORT}: {e}");
                    return;
                }
            };
            let flag = Arc::new(AtomicBool::new(true));
            let (app2, flag2) = (app.clone(), flag.clone());
            std::thread::Builder::new()
                .name("agent-server".into())
                .spawn(move || serve(app2, listener, flag2))
                .ok();
            *running = Some(flag);
        }
        (false, true) => {
            if let Some(flag) = running.take() {
                flag.store(false, Ordering::Relaxed);
                // Despierta al accept() bloqueado para que el hilo termine.
                let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], PORT).into(), Duration::from_millis(200));
            }
            st.sessions.lock().unwrap().retain(|_, s| s.agent != CLAUDE);
            emit(app);
        }
        _ => {}
    }
}

fn serve(app: AppHandle, listener: TcpListener, flag: Arc<AtomicBool>) {
    for conn in listener.incoming() {
        if !flag.load(Ordering::Relaxed) {
            break;
        }
        if let Ok(mut s) = conn {
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
            if let Some((path, body)) = read_request(&mut s) {
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
                drop(s);
                if let Ok(v) = serde_json::from_slice::<Value>(&body) {
                    let changed = match path.as_str() {
                        "/hook" => on_hook(&app, &v),
                        "/status" => on_status(&app, &v),
                        _ => false,
                    };
                    if changed {
                        emit(&app);
                    }
                }
            } else {
                let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
        }
    }
}

/// HTTP/1.1 mínimo: línea de petición, cabeceras y cuerpo por Content-Length.
fn read_request(s: &mut TcpStream) -> Option<(String, Vec<u8>)> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if buf.len() > 64 * 1024 {
            return None;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let req = lines.next()?;
    let mut parts = req.split_whitespace();
    if parts.next()? != "POST" {
        return None;
    }
    let path = parts.next()?.split('?').next()?.to_string();
    let len: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    if len > 8 * 1024 * 1024 {
        return None;
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < len {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Some((path, body))
}

fn str_of<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str())
}

pub(crate) fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

pub(crate) fn truncate(s: &str, n: usize) -> String {
    let s = s.trim().replace(['\n', '\r'], " ");
    if s.chars().count() <= n {
        s
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// Resumen corto de una llamada a herramienta.
fn describe_tool(name: &str, input: &Value) -> String {
    let fp = input.get("file_path").or_else(|| input.get("notebook_path")).and_then(|x| x.as_str());
    match (name, fp) {
        (_, Some(p)) => format!("{name} {}", file_name(p)),
        ("Bash" | "PowerShell", _) => format!("{name} {}", truncate(str_of(input, "description").or(str_of(input, "command")).unwrap_or(""), 48)),
        ("Grep" | "Glob", _) => format!("{name} {}", truncate(str_of(input, "pattern").unwrap_or(""), 40)),
        ("WebFetch", _) => format!("{name} {}", truncate(str_of(input, "url").unwrap_or(""), 40)),
        ("Agent" | "Task", _) => format!("{name} {}", truncate(str_of(input, "description").unwrap_or(""), 40)),
        _ => name.to_string(),
    }
}

fn session_entry<'a>(map: &'a mut HashMap<String, AgentSession>, v: &Value) -> Option<&'a mut AgentSession> {
    let id = str_of(v, "session_id")?.to_string();
    let s = map.entry(id.clone()).or_insert_with(|| AgentSession { id, agent: CLAUDE.into(), ..Default::default() });
    if let Some(cwd) = str_of(v, "cwd") {
        s.cwd = cwd.to_string();
        s.project = file_name(cwd.trim_end_matches(['/', '\\']));
    }
    s.last_event_ms = now_ms();
    Some(s)
}

/// Aplica un evento de hook. Devuelve si cambió algo visible.
pub fn apply_hook(map: &mut HashMap<String, AgentSession>, v: &Value) -> bool {
    let event = str_of(v, "hook_event_name").unwrap_or("").to_string();
    // Los subagentes comparten session_id: sus eventos cuentan como actividad de la sesión.
    if event == "SessionEnd" {
        return str_of(v, "session_id").and_then(|id| map.remove(id)).is_some();
    }
    let Some(s) = session_entry(map, v) else { return false };
    let now = s.last_event_ms;
    match event.as_str() {
        "SessionStart" => {
            if let Some(m) = str_of(v, "model") {
                s.usage.model = Some(m.to_string());
            }
            if s.status != Status::Working {
                s.status = Status::Idle;
            }
        }
        "UserPromptSubmit" => {
            s.status = Status::Working;
            s.turn_started_ms = Some(now);
            s.turn_ended_ms = None;
            s.files.clear();
            s.tool_count = 0;
            s.message = str_of(v, "prompt_text").or(str_of(v, "prompt")).map(|p| truncate(p, 120));
            s.last_action = None;
        }
        "PreToolUse" => {
            s.status = Status::Working;
            if s.turn_started_ms.is_none() {
                s.turn_started_ms = Some(now);
            }
            let name = str_of(v, "tool_name").unwrap_or("Herramienta");
            let input = v.get("tool_input").cloned().unwrap_or(Value::Null);
            s.last_action = Some(describe_tool(name, &input));
            s.tool_count += 1;
            if matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
                if let Some(p) = input.get("file_path").or_else(|| input.get("notebook_path")).and_then(|x| x.as_str()) {
                    let f = file_name(p);
                    s.files.retain(|x| x != &f);
                    s.files.insert(0, f);
                    s.files.truncate(MAX_FILES);
                }
            }
        }
        "PostToolUse" | "PostToolUseFailure" | "SubagentStart" | "SubagentStop" => {
            // Tras aprobar un permiso, el siguiente evento devuelve la sesión a "trabajando".
            if s.status == Status::Waiting {
                s.status = Status::Working;
            }
        }
        "Notification" => {
            let kind = str_of(v, "notification_type").unwrap_or("");
            let msg = str_of(v, "message").map(|m| truncate(m, 120));
            match kind {
                "idle_prompt" => {
                    if s.status != Status::Done {
                        s.status = Status::Idle;
                    }
                }
                _ => {
                    s.status = Status::Waiting;
                    s.message = msg;
                }
            }
        }
        "Stop" => {
            s.status = Status::Done;
            s.turn_ended_ms = Some(now);
            if let Some(m) = str_of(v, "last_assistant_message") {
                s.message = Some(truncate(m, 160));
            }
        }
        "StopFailure" => {
            s.status = Status::Error;
            s.turn_ended_ms = Some(now);
            s.message = str_of(v, "error").or(str_of(v, "message")).map(|m| truncate(m, 160));
        }
        _ => return false,
    }
    true
}

/// Aplica el JSON del statusLine (tokens, coste y límites).
pub fn apply_status(map: &mut HashMap<String, AgentSession>, v: &Value) -> bool {
    let Some(s) = session_entry(map, v) else { return false };
    let f = |p: &str| v.pointer(p).and_then(|x| x.as_f64());
    let u = |p: &str| v.pointer(p).and_then(|x| x.as_u64());
    let i = |p: &str| v.pointer(p).and_then(|x| x.as_i64());
    s.usage = Usage {
        model: v.pointer("/model/display_name").and_then(|x| x.as_str()).map(String::from).or(s.usage.model.take()),
        context_pct: f("/context_window/used_percentage"),
        context_size: u("/context_window/context_window_size"),
        input_tokens: u("/context_window/total_input_tokens"),
        output_tokens: u("/context_window/total_output_tokens"),
        cost_usd: f("/cost/total_cost_usd"),
        lines_added: u("/cost/total_lines_added"),
        lines_removed: u("/cost/total_lines_removed"),
        limits: vec![
            Limit { label: "5 horas".into(), short: "5h".into(), pct: f("/rate_limits/five_hour/used_percentage"), resets: i("/rate_limits/five_hour/resets_at") },
            Limit { label: "Semana".into(), short: "Sem".into(), pct: f("/rate_limits/seven_day/used_percentage"), resets: i("/rate_limits/seven_day/resets_at") },
        ],
    };
    true
}

fn on_hook(app: &AppHandle, v: &Value) -> bool {
    apply_hook(&mut app.state::<AgentState>().sessions.lock().unwrap(), v)
}

fn on_status(app: &AppHandle, v: &Value) -> bool {
    apply_status(&mut app.state::<AgentState>().sessions.lock().unwrap(), v)
}

/// Sesiones ordenadas por última actividad (la primera es la que se muestra).
pub fn snapshot(app: &AppHandle) -> Vec<AgentSession> {
    let st = app.state::<AgentState>();
    let mut list: Vec<AgentSession> = st.sessions.lock().unwrap().values().cloned().collect();
    list.sort_by_key(|s| std::cmp::Reverse(s.last_event_ms));
    list
}

pub(crate) fn emit(app: &AppHandle) {
    let _ = app.emit("agent://state", snapshot(app));
}
