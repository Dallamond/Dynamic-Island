// Temporizadores: el estado vive en Rust; aquí solo se dibuja y se calcula lo que queda.
// No hay icono de tomate en lucide: la manzana hace de "pomodoro".
import { Apple as Tomato, BellRing, Hourglass, Pause, Play, RotateCcw, SkipForward, Timer as TimerIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useActivity } from "../../core/activities";
import { useTauriEvent } from "../../core/ipc";
import { createStore, useStore } from "../../core/store";

type Phase = "work" | "shortBreak" | "longBreak";
interface TimersState {
  stopwatch: { running: boolean; accumulatedMs: number; startedAtMs: number | null };
  countdown: { durationMs: number; running: boolean; remainingMs: number; endsAtMs: number | null };
  pomodoro: {
    phase: Phase;
    running: boolean;
    remainingMs: number;
    endsAtMs: number | null;
    completed: number;
    workMin: number;
    shortMin: number;
    longMin: number;
  };
}
interface TimerDone {
  kind: string;
  message: string;
}

const timersStore = createStore<TimersState | null>(null);
const doneStore = createStore<{ msg: string; at: number } | null>(null);

const act = (action: string, extra: Record<string, unknown> = {}) =>
  invoke<TimersState>("timer_action", { action: { action, ...extra } }).then(timersStore.set);

const PHASE: Record<Phase, string> = { work: "Concentración", shortBreak: "Descanso", longBreak: "Descanso largo" };

function fmt(ms: number, withTenths = false) {
  const t = Math.max(0, ms);
  const h = Math.floor(t / 3_600_000);
  const m = Math.floor((t % 3_600_000) / 60_000);
  const s = Math.floor((t % 60_000) / 1000);
  const base = `${h > 0 ? `${h}:${m.toString().padStart(2, "0")}` : m}:${s.toString().padStart(2, "0")}`;
  return withTenths ? `${base}.${Math.floor((t % 1000) / 100)}` : base;
}

const pomoLeft = (s: TimersState, now: number) => (s.pomodoro.running && s.pomodoro.endsAtMs ? s.pomodoro.endsAtMs - now : s.pomodoro.remainingMs);
const cdLeft = (s: TimersState, now: number) => (s.countdown.running && s.countdown.endsAtMs ? s.countdown.endsAtMs - now : s.countdown.remainingMs);
const swElapsed = (s: TimersState, now: number) => s.stopwatch.accumulatedMs + (s.stopwatch.running && s.stopwatch.startedAtMs ? now - s.stopwatch.startedAtMs : 0);

function useNow(active: boolean, every = 1000) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now()); // sin esto, el primer render usa la hora del montaje
    const t = window.setInterval(() => setNow(Date.now()), every);
    return () => window.clearInterval(t);
  }, [active, every]);
  return active ? Date.now() : now;
}

/** Campanilla corta con WebAudio (sin ficheros de sonido). */
function chime() {
  try {
    const ctx = new AudioContext();
    [880, 1175, 1568].forEach((f, i) => {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.frequency.value = f;
      o.type = "sine";
      const t0 = ctx.currentTime + i * 0.18;
      g.gain.setValueAtTime(0.0001, t0);
      g.gain.exponentialRampToValueAtTime(0.25, t0 + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.6);
      o.connect(g).connect(ctx.destination);
      o.start(t0);
      o.stop(t0 + 0.65);
    });
    window.setTimeout(() => ctx.close(), 1500);
  } catch {}
}

// ---------------------------------------------------------------- Host

const DONE_MS = 10_000;

export function TimerHost() {
  useTauriEvent<TimersState>("timer://state", timersStore.set);
  useTauriEvent<TimerDone>("timer://done", (d) => {
    doneStore.set({ msg: d.message, at: Date.now() });
    chime();
    window.setTimeout(() => {
      if (Date.now() - (doneStore.get()?.at ?? 0) >= DONE_MS) doneStore.set(null);
    }, DONE_MS + 20);
  });
  useEffect(() => {
    invoke<TimersState>("timer_state").then(timersStore.set);
  }, []);

  const s = useStore(timersStore);
  const done = useStore(doneStore);
  const running = !!s && (s.pomodoro.running || s.countdown.running || s.stopwatch.running);

  useActivity(done ? { id: "timer-done", priority: 40, Compact: DoneCompact, tab: "timer" } : null);
  useActivity(running ? { id: "timer", priority: 18, Compact: TimerCompact, Badge: TimerBadge, tab: "timer" } : null);
  return null;
}

function DoneCompact() {
  const d = useStore(doneStore);
  return (
    <span className="timer-compact done">
      <BellRing size={15} className="agent-pulse" />
      <span>{d?.msg}</span>
    </span>
  );
}

/** Qué temporizador enseñar en la píldora: pomodoro > cuenta atrás > cronómetro. */
function primary(s: TimersState, now: number) {
  if (s.pomodoro.running) return { icon: Tomato, label: PHASE[s.pomodoro.phase], value: fmt(pomoLeft(s, now)) };
  if (s.countdown.running) return { icon: Hourglass, label: "Cuenta atrás", value: fmt(cdLeft(s, now)) };
  return { icon: TimerIcon, label: "Cronómetro", value: fmt(swElapsed(s, now)) };
}

function TimerCompact() {
  const s = useStore(timersStore);
  const now = useNow(true);
  if (!s) return null;
  const p = primary(s, now);
  return (
    <span className="timer-compact">
      <p.icon size={15} />
      <span className="muted">{p.label}</span>
      <span className="timer-compact-value">{p.value}</span>
    </span>
  );
}

function TimerBadge() {
  const s = useStore(timersStore);
  const now = useNow(true);
  if (!s) return null;
  const p = primary(s, now);
  return (
    <span className="timer-badge" title={p.label}>
      <p.icon size={13} />
      {p.value}
    </span>
  );
}

// ---------------------------------------------------------------- Panel

type Tab = "pomodoro" | "countdown" | "stopwatch";
const PRESETS = [1, 5, 10, 15, 25, 45, 60];

export function TimerPanel() {
  const s = useStore(timersStore);
  const [tab, setTab] = useState<Tab>(() => (s?.countdown.running ? "countdown" : s?.stopwatch.running && !s.pomodoro.running ? "stopwatch" : "pomodoro"));
  const running = !!s && ((tab === "pomodoro" && s.pomodoro.running) || (tab === "countdown" && s.countdown.running) || (tab === "stopwatch" && s.stopwatch.running));
  const now = useNow(running, tab === "stopwatch" ? 100 : 1000);
  const [custom, setCustom] = useState("");
  if (!s) return null;

  let big = "";
  let caption = "";
  let progress = 0;
  if (tab === "pomodoro") {
    const left = pomoLeft(s, now);
    const total = (s.pomodoro.phase === "work" ? s.pomodoro.workMin : s.pomodoro.phase === "shortBreak" ? s.pomodoro.shortMin : s.pomodoro.longMin) * 60_000;
    big = fmt(left);
    caption = `${PHASE[s.pomodoro.phase]} · ${s.pomodoro.completed} hechos`;
    progress = 1 - left / total;
  } else if (tab === "countdown") {
    const left = cdLeft(s, now);
    big = fmt(left);
    caption = s.countdown.durationMs > 0 ? `de ${fmt(s.countdown.durationMs)}` : "Elige un tiempo";
    progress = s.countdown.durationMs > 0 ? 1 - left / s.countdown.durationMs : 0;
  } else {
    big = fmt(swElapsed(s, now), true);
    caption = "Cronómetro";
  }

  const toggle = () => act(tab === "pomodoro" ? "pomodoroToggle" : tab === "countdown" ? "countdownToggle" : "stopwatchToggle");
  const reset = () => act(tab === "pomodoro" ? "pomodoroReset" : tab === "countdown" ? "countdownReset" : "stopwatchReset");

  return (
    <div className="timer-panel">
      <div className="seg">
        {(["pomodoro", "countdown", "stopwatch"] as Tab[]).map((t) => (
          <button key={t} className={tab === t ? "on" : ""} onClick={() => setTab(t)}>
            {t === "pomodoro" ? "Pomodoro" : t === "countdown" ? "Cuenta atrás" : "Cronómetro"}
          </button>
        ))}
      </div>

      <div className="timer-main">
        <div className="timer-big">{big}</div>
        <div className="muted timer-caption">{caption}</div>
        {tab !== "stopwatch" && (
          <div className="timer-progress">
            <div style={{ width: `${Math.min(100, Math.max(0, progress * 100))}%` }} />
          </div>
        )}
      </div>

      {tab === "countdown" && !s.countdown.running && (
        <div className="timer-presets">
          {PRESETS.map((m) => (
            <button key={m} onClick={() => act("countdownSet", { minutes: m })}>
              {m}m
            </button>
          ))}
          <input
            placeholder="min"
            value={custom}
            inputMode="decimal"
            onChange={(e) => setCustom(e.target.value.replace(/[^\d.,]/g, ""))}
            onKeyDown={(e) => {
              const v = parseFloat(custom.replace(",", "."));
              if (e.key === "Enter" && v > 0) {
                act("countdownSet", { minutes: v });
                setCustom("");
              }
            }}
          />
        </div>
      )}

      <div className="timer-controls">
        <button className="btn" style={{ width: 34, height: 34 }} title="Reiniciar" onClick={reset}>
          <RotateCcw size={17} />
        </button>
        <button className="btn timer-play" style={{ width: 46, height: 46 }} onClick={toggle}>
          {running ? <Pause size={22} fill="currentColor" /> : <Play size={22} fill="currentColor" />}
        </button>
        {tab === "pomodoro" ? (
          <button className="btn" style={{ width: 34, height: 34 }} title="Saltar fase" onClick={() => act("pomodoroSkip")}>
            <SkipForward size={17} />
          </button>
        ) : (
          <span style={{ width: 34 }} />
        )}
      </div>
    </div>
  );
}
