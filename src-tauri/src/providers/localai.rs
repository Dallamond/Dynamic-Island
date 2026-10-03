//! IA local (Bionic / LM Studio u Ollama) por su API HTTP en un endpoint configurable.
//!
//! Ninguna de las dos APIs dice "estoy generando": se pregunta qué modelo hay cargado
//! (`/api/v0/models` de LM Studio/Bionic, `/api/ps` de Ollama) y "generando" se deduce de la carga
//! de la GPU por NVML. Si la GPU se usa para otra cosa con el modelo cargado, también contará.
//!
//! Consumo: una petición a localhost cada 2 s con el servidor encendido, cada 10 s si no responde.
//! NVML solo se consulta cuando hay un modelo cargado.

use super::agent::{self, now_ms, AgentSession, AgentState, Status};
use nvml_wrapper::Nvml;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub const KIND: &str = "local";
/// Uso de GPU a partir del cual se considera que el modelo está generando (reposo medido: ≤ 18 %).
pub const GPU_BUSY_PCT: u32 = 60;
/// Lecturas seguidas por encima del umbral para pasar a "generando" (evita picos sueltos).
const BUSY_SAMPLES: u8 = 2;
const POLL_OK: Duration = Duration::from_secs(2);
const POLL_DOWN: Duration = Duration::from_secs(10);

#[derive(Clone, PartialEq, Debug)]
pub struct Config {
    pub endpoint: String,
    pub name: String,
}

static RUNNING: Mutex<Option<(Config, Arc<AtomicBool>)>> = Mutex::new(None);

pub fn set_enabled(app: &AppHandle, enabled: bool, cfg: Config) {
    let mut running = RUNNING.lock().unwrap();
    let same = running.as_ref().is_some_and(|(c, _)| *c == cfg);
    if enabled && same {
        return;
    }
    if let Some((_, flag)) = running.take() {
        flag.store(false, Ordering::Relaxed);
        app.state::<AgentState>().sessions.lock().unwrap().retain(|_, s| s.kind != KIND);
        agent::emit(app);
    }
    if enabled {
        let flag = Arc::new(AtomicBool::new(true));
        let (app2, flag2, cfg2) = (app.clone(), flag.clone(), cfg.clone());
        std::thread::Builder::new()
            .name("localai-watch".into())
            .spawn(move || watch(app2, cfg2, flag2))
            .ok();
        *running = Some((cfg, flag));
    }
}

/// Modelo cargado según la API.
#[derive(Clone, PartialEq, Debug)]
pub struct Loaded {
    pub server: &'static str,
    pub model: String,
    pub context: Option<u64>,
}

/// Seguimiento entre lecturas: cuántas seguidas lleva la GPU ocupada.
#[derive(Default)]
pub struct Tracker {
    hot: u8,
}

fn watch(app: AppHandle, cfg: Config, flag: Arc<AtomicBool>) {
    let id = format!("local:{}", cfg.endpoint);
    let mut nvml: Option<Nvml> = None;
    let mut tracker = Tracker::default();
    while flag.load(Ordering::Relaxed) {
        let loaded = query(&cfg.endpoint);
        let gpu = loaded.as_ref().and_then(|_| {
            if nvml.is_none() {
                nvml = Nvml::init().ok();
            }
            gpu_pct(nvml.as_ref()?)
        });
        if loaded.is_none() {
            nvml = None; // libera NVML mientras no haya modelo
        }
        let changed = {
            let st = app.state::<AgentState>();
            let mut map = st.sessions.lock().unwrap();
            match &loaded {
                Some(l) => {
                    let name = if cfg.name.trim().is_empty() { l.server.to_string() } else { cfg.name.trim().to_string() };
                    let s = map.entry(id.clone()).or_insert_with(|| AgentSession { id: id.clone(), kind: KIND.into(), ..Default::default() });
                    s.agent = name;
                    step(s, &mut tracker, l, gpu, now_ms())
                }
                None => map.remove(&id).is_some(),
            }
        };
        if changed {
            agent::emit(&app);
        }
        std::thread::sleep(if loaded.is_some() { POLL_OK } else { POLL_DOWN });
    }
}

fn gpu_pct(nvml: &Nvml) -> Option<u32> {
    nvml.device_by_index(0).ok()?.utilization_rates().ok().map(|u| u.gpu)
}

/// Aplica una lectura. Devuelve si cambió algo visible.
pub fn step(s: &mut AgentSession, t: &mut Tracker, l: &Loaded, gpu: Option<u32>, now: i64) -> bool {
    let mut changed = false;
    if s.usage.model.as_deref() != Some(&l.model) || s.project != l.model {
        s.usage.model = Some(l.model.clone());
        s.project = l.model.clone();
        s.usage.context_size = l.context;
        changed = true;
    }
    let busy = gpu.is_some_and(|g| g >= GPU_BUSY_PCT);
    t.hot = if busy { t.hot.saturating_add(1) } else { 0 };
    let pct = gpu.map(f64::from);
    if s.usage.gpu_pct != pct {
        s.usage.gpu_pct = pct;
        changed = true;
    }
    if t.hot >= BUSY_SAMPLES && s.status != Status::Working {
        s.status = Status::Working;
        s.turn_started_ms = Some(now);
        s.turn_ended_ms = None;
        s.last_action = Some("Generando".into());
        s.message = None;
        s.last_event_ms = now;
        changed = true;
    } else if !busy && s.status == Status::Working {
        s.status = Status::Done;
        s.turn_ended_ms = Some(now);
        s.last_action = None;
        s.message = Some("Respuesta terminada".into());
        s.last_event_ms = now;
        changed = true;
    }
    changed
}

/// Pregunta al endpoint: primero API de LM Studio/Bionic, luego Ollama.
fn query(endpoint: &str) -> Option<Loaded> {
    if let Some(v) = get_json(endpoint, "/api/v0/models") {
        return parse_lmstudio(&v);
    }
    get_json(endpoint, "/api/ps").and_then(|v| parse_ollama(&v))
}

/// `/api/v0/models` de LM Studio/Bionic: el primer modelo de texto en estado "loaded".
pub fn parse_lmstudio(v: &Value) -> Option<Loaded> {
    let m = v.get("data")?.as_array()?.iter().find(|m| {
        m.get("state").and_then(|x| x.as_str()) == Some("loaded") && m.get("type").and_then(|x| x.as_str()) != Some("embeddings")
    })?;
    Some(Loaded {
        server: "LM Studio",
        model: m.get("id")?.as_str()?.to_string(),
        context: m.get("loaded_context_length").and_then(|x| x.as_u64()),
    })
}

/// `/api/ps` de Ollama: el primer modelo cargado.
pub fn parse_ollama(v: &Value) -> Option<Loaded> {
    let m = v.get("models")?.as_array()?.first()?;
    Some(Loaded {
        server: "Ollama",
        model: m.get("name").or_else(|| m.get("model"))?.as_str()?.to_string(),
        context: m.get("context_length").and_then(|x| x.as_u64()),
    })
}

/// GET mínimo por HTTP/1.0 (sin chunked) a un endpoint `http://host:puerto`.
fn get_json(endpoint: &str, path: &str) -> Option<Value> {
    let hostport = endpoint.trim().trim_start_matches("http://").trim_end_matches('/');
    let addr = hostport.to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(400)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(s, "GET {path} HTTP/1.0\r\nHost: {hostport}\r\nAccept: application/json\r\n\r\n").ok()?;
    let mut buf = Vec::new();
    s.take(4 * 1024 * 1024).read_to_end(&mut buf).ok()?;
    let i = buf.windows(4).position(|w| w == b"\r\n\r\n")?;
    let status = String::from_utf8_lossy(&buf[..i]);
    if !status.split_whitespace().nth(1).is_some_and(|c| c == "200") {
        return None;
    }
    serde_json::from_slice(&buf[i + 4..]).ok()
}

/// Lectura puntual del endpoint y de la GPU (diagnóstico y tests en vivo).
pub fn probe(endpoint: &str) -> (Option<Loaded>, Option<u32>) {
    let loaded = query(endpoint);
    let gpu = Nvml::init().ok().and_then(|n| gpu_pct(&n));
    (loaded, gpu)
}
