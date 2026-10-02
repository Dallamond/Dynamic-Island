//! Pomodoro, cronómetro y cuenta atrás. Viven en Rust para que sigan con la isla cerrada.
//!
//! No hay "tick": el estado son marcas de tiempo y el frontend calcula lo que muestra.
//! Un único hilo duerme hasta el siguiente vencimiento (condvar) y entonces avisa.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stopwatch {
    pub running: bool,
    /// Acumulado antes del último arranque.
    pub accumulated_ms: i64,
    pub started_at_ms: Option<i64>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub duration_ms: i64,
    pub running: bool,
    /// Restante cuando está en pausa (o duración si no ha empezado).
    pub remaining_ms: i64,
    pub ends_at_ms: Option<i64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Pomodoro {
    pub phase: Phase,
    pub running: bool,
    pub remaining_ms: i64,
    pub ends_at_ms: Option<i64>,
    /// Pomodoros de trabajo completados.
    pub completed: u32,
    pub work_min: u32,
    pub short_min: u32,
    pub long_min: u32,
    pub rounds_before_long: u32,
}

impl Default for Pomodoro {
    fn default() -> Self {
        Self { phase: Phase::Work, running: false, remaining_ms: 25 * 60_000, ends_at_ms: None, completed: 0, work_min: 25, short_min: 5, long_min: 15, rounds_before_long: 4 }
    }
}

impl Pomodoro {
    fn phase_ms(&self, p: Phase) -> i64 {
        60_000 * match p {
            Phase::Work => self.work_min,
            Phase::ShortBreak => self.short_min,
            Phase::LongBreak => self.long_min,
        } as i64
    }
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimersState {
    pub stopwatch: Stopwatch,
    pub countdown: Countdown,
    pub pomodoro: Pomodoro,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerDone {
    pub kind: String,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum TimerAction {
    StopwatchToggle,
    StopwatchReset,
    CountdownSet { minutes: f64 },
    CountdownToggle,
    CountdownReset,
    PomodoroToggle,
    PomodoroReset,
    PomodoroSkip,
    PomodoroConfig { work: u32, short: u32, long: u32 },
}

#[derive(Default)]
pub struct Timers {
    pub state: Mutex<TimersState>,
    wake: Condvar,
}

/// Lógica pura (testeable): aplica una acción en el instante `now`.
pub fn apply(s: &mut TimersState, a: TimerAction, now: i64) {
    match a {
        TimerAction::StopwatchToggle => {
            let sw = &mut s.stopwatch;
            if sw.running {
                sw.accumulated_ms += now - sw.started_at_ms.unwrap_or(now);
                sw.started_at_ms = None;
            } else {
                sw.started_at_ms = Some(now);
            }
            sw.running = !sw.running;
        }
        TimerAction::StopwatchReset => s.stopwatch = Stopwatch::default(),
        TimerAction::CountdownSet { minutes } => {
            let ms = (minutes.max(0.0) * 60_000.0) as i64;
            s.countdown = Countdown { duration_ms: ms, running: false, remaining_ms: ms, ends_at_ms: None };
        }
        TimerAction::CountdownToggle => {
            let c = &mut s.countdown;
            if c.running {
                c.remaining_ms = (c.ends_at_ms.unwrap_or(now) - now).max(0);
                c.ends_at_ms = None;
                c.running = false;
            } else if c.remaining_ms > 0 {
                c.ends_at_ms = Some(now + c.remaining_ms);
                c.running = true;
            }
        }
        TimerAction::CountdownReset => {
            let d = s.countdown.duration_ms;
            s.countdown = Countdown { duration_ms: d, running: false, remaining_ms: d, ends_at_ms: None };
        }
        TimerAction::PomodoroToggle => {
            let p = &mut s.pomodoro;
            if p.running {
                p.remaining_ms = (p.ends_at_ms.unwrap_or(now) - now).max(0);
                p.ends_at_ms = None;
            } else {
                p.ends_at_ms = Some(now + p.remaining_ms);
            }
            p.running = !p.running;
        }
        TimerAction::PomodoroReset => {
            let p = &s.pomodoro;
            let (w, sh, l, r) = (p.work_min, p.short_min, p.long_min, p.rounds_before_long);
            s.pomodoro = Pomodoro { work_min: w, short_min: sh, long_min: l, rounds_before_long: r, remaining_ms: w as i64 * 60_000, ..Default::default() };
        }
        TimerAction::PomodoroSkip => {
            next_phase(&mut s.pomodoro, now);
        }
        TimerAction::PomodoroConfig { work, short, long } => {
            let p = &mut s.pomodoro;
            p.work_min = work.clamp(1, 180);
            p.short_min = short.clamp(1, 60);
            p.long_min = long.clamp(1, 120);
            if !p.running {
                p.remaining_ms = p.phase_ms(p.phase);
            }
        }
    }
}

/// Pasa a la siguiente fase del pomodoro y la arranca (si estaba corriendo).
fn next_phase(p: &mut Pomodoro, now: i64) {
    p.phase = match p.phase {
        Phase::Work => {
            p.completed += 1;
            if p.completed % p.rounds_before_long.max(1) == 0 {
                Phase::LongBreak
            } else {
                Phase::ShortBreak
            }
        }
        _ => Phase::Work,
    };
    p.remaining_ms = p.phase_ms(p.phase);
    p.ends_at_ms = if p.running { Some(now + p.remaining_ms) } else { None };
}

/// Revisa vencimientos en `now`. Devuelve los avisos a emitir.
pub fn check_deadlines(s: &mut TimersState, now: i64) -> Vec<TimerDone> {
    let mut done = Vec::new();
    let c = &mut s.countdown;
    if c.running && c.ends_at_ms.is_some_and(|e| e <= now) {
        c.running = false;
        c.ends_at_ms = None;
        c.remaining_ms = 0;
        done.push(TimerDone { kind: "countdown".into(), message: "Cuenta atrás terminada".into() });
    }
    let p = &mut s.pomodoro;
    if p.running && p.ends_at_ms.is_some_and(|e| e <= now) {
        let msg = match p.phase {
            Phase::Work => "¡Pomodoro hecho! Toca descanso",
            _ => "Descanso terminado: a trabajar",
        };
        next_phase(p, now);
        done.push(TimerDone { kind: "pomodoro".into(), message: msg.into() });
    }
    done
}

fn next_deadline(s: &TimersState) -> Option<i64> {
    [s.countdown.ends_at_ms.filter(|_| s.countdown.running), s.pomodoro.ends_at_ms.filter(|_| s.pomodoro.running)]
        .into_iter()
        .flatten()
        .min()
}

pub fn action(app: &AppHandle, a: TimerAction) -> TimersState {
    let t = app.state::<Arc<Timers>>();
    let snapshot = {
        let mut s = t.state.lock().unwrap();
        apply(&mut s, a, now_ms());
        s.clone()
    };
    t.wake.notify_all();
    let _ = app.emit("timer://state", snapshot.clone());
    snapshot
}

pub fn spawn(app: AppHandle) {
    let t = app.state::<Arc<Timers>>().inner().clone();
    std::thread::Builder::new()
        .name("timers".into())
        .spawn(move || {
            let mut guard = t.state.lock().unwrap();
            loop {
                let wait = next_deadline(&guard).map(|d| (d - now_ms()).max(0) as u64);
                guard = match wait {
                    // Nada en marcha: duerme hasta que una acción lo despierte.
                    None => t.wake.wait(guard).unwrap(),
                    Some(ms) => t.wake.wait_timeout(guard, Duration::from_millis(ms + 5)).unwrap().0,
                };
                let done = check_deadlines(&mut guard, now_ms());
                if !done.is_empty() {
                    let snap = guard.clone();
                    drop(guard);
                    for d in done {
                        let _ = app.emit("timer://done", d);
                    }
                    let _ = app.emit("timer://state", snap);
                    guard = t.state.lock().unwrap();
                }
            }
        })
        .ok();
}
