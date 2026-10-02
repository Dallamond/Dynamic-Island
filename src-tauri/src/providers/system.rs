//! CPU, RAM y GPU. Solo sondea mientras el panel de sistema está abierto (`system_watch(true)`).
//! GPU NVIDIA por NVML (nvml.dll del driver); si no hay NVML, la GPU simplemente no aparece.

use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, Manager};

const INTERVAL: Duration = Duration::from_millis(1000);
/// Los procesos son lo más caro de refrescar: uno de cada N ciclos.
const PROCESS_EVERY: u32 = 3;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub name: String,
    pub usage: u32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub temp: Option<u32>,
    pub power_w: Option<f32>,
    pub fan: Option<u32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Proc {
    pub name: String,
    pub cpu: f32,
    pub mem: u64,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu: f32,
    pub cpu_name: String,
    pub cores: usize,
    pub freq_mhz: u64,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub gpu: Option<Gpu>,
    pub top: Vec<Proc>,
}

#[derive(Default)]
pub struct SystemState {
    watching: Mutex<Option<Arc<AtomicBool>>>,
}

pub fn watch(app: &AppHandle, on: bool) {
    let st = app.state::<SystemState>();
    let mut w = st.watching.lock().unwrap();
    match (on, w.is_some()) {
        (true, false) => {
            let flag = Arc::new(AtomicBool::new(true));
            let (app2, flag2) = (app.clone(), flag.clone());
            std::thread::Builder::new().name("system".into()).spawn(move || run(app2, flag2)).ok();
            *w = Some(flag);
        }
        (false, true) => {
            if let Some(f) = w.take() {
                f.store(false, Ordering::Relaxed);
            }
        }
        _ => {}
    }
}

fn run(app: AppHandle, flag: Arc<AtomicBool>) {
    let mut sys = System::new();
    let nvml = Nvml::init().ok();
    let mut tick = 0u32;
    let mut top: Vec<Proc> = Vec::new();
    sys.refresh_cpu_all();
    let cpu_name = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    while flag.load(Ordering::Relaxed) {
        std::thread::sleep(INTERVAL);
        if !flag.load(Ordering::Relaxed) {
            break;
        }
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let cores = sys.cpus().len().max(1);
        // El uso de CPU por proceso necesita dos muestras: la primera solo sirve de base.
        if tick < 2 || tick % PROCESS_EVERY == 0 {
            sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());
            // Agrupa por nombre (Chrome, Spotify... tienen muchos procesos).
            let mut agg: std::collections::HashMap<String, (f32, u64)> = Default::default();
            for p in sys.processes().values() {
                let name = p.name().to_string_lossy().trim_end_matches(".exe").to_string();
                if name == "System Idle Process" || name == "Idle" {
                    continue;
                }
                let e = agg.entry(name).or_default();
                e.0 += p.cpu_usage() / cores as f32;
                e.1 += p.memory();
            }
            let mut list: Vec<Proc> = agg.into_iter().map(|(name, (cpu, mem))| Proc { name, cpu, mem }).collect();
            list.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
            list.truncate(4);
            if tick > 0 {
                top = list;
            }
        }
        tick = tick.wrapping_add(1);
        let stats = SystemStats {
            cpu: sys.global_cpu_usage(),
            cpu_name: cpu_name.clone(),
            cores,
            freq_mhz: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
            mem_used: sys.used_memory(),
            mem_total: sys.total_memory(),
            swap_used: sys.used_swap(),
            swap_total: sys.total_swap(),
            gpu: nvml.as_ref().and_then(read_gpu),
            top: top.clone(),
        };
        let _ = app.emit("system://stats", stats);
    }
}

fn read_gpu(nvml: &Nvml) -> Option<Gpu> {
    let d = nvml.device_by_index(0).ok()?;
    let mem = d.memory_info().ok()?;
    Some(Gpu {
        name: d.name().unwrap_or_default().replace("NVIDIA GeForce ", ""),
        usage: d.utilization_rates().map(|u| u.gpu).unwrap_or(0),
        mem_used: mem.used,
        mem_total: mem.total,
        temp: d.temperature(TemperatureSensor::Gpu).ok(),
        power_w: d.power_usage().ok().map(|mw| mw as f32 / 1000.0),
        fan: d.fan_speed(0).ok(),
    })
}
