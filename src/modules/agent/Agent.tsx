// Módulo de Claude Code: estado del turno en la píldora y detalle intermedio en el panel.
import { AlertTriangle, Check, FileCode2, Hand, Sparkle } from "lucide-react";
import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useActivity } from "../../core/activities";
import { useTauriEvent } from "../../core/ipc";
import { createStore, useStore } from "../../core/store";

type Status = "idle" | "working" | "waiting" | "done" | "error";

interface Usage {
  model: string | null;
  contextPct: number | null;
  contextSize: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
  costUsd: number | null;
  linesAdded: number | null;
  linesRemoved: number | null;
  fiveHourPct: number | null;
  fiveHourResets: number | null;
  sevenDayPct: number | null;
  sevenDayResets: number | null;
}

interface AgentSession {
  id: string;
  agent: string;
  project: string;
  cwd: string;
  status: Status;
  lastAction: string | null;
  message: string | null;
  files: string[];
  toolCount: number;
  turnStartedMs: number | null;
  turnEndedMs: number | null;
  lastEventMs: number;
  usage: Usage;
}

const agentStore = createStore<AgentSession[]>([]);

/** "Listo" y "error" se quedan en la píldora un rato y luego vuelve el reloj. */
const DONE_VISIBLE_MS = 12_000;
const ERROR_VISIBLE_MS = 30_000;
const CLAUDE = "#d97757";

// ---------------------------------------------------------------- Host

export function AgentHost() {
  useTauriEvent<AgentSession[]>("agent://state", agentStore.set);
  useEffect(() => {
    invoke<AgentSession[]>("agent_state").then(agentStore.set);
  }, []);

  const sessions = useStore(agentStore);
  const s = sessions[0];
  const [, force] = useState(0);

  // Caducidad de "listo"/"error": un único timeout, solo si hace falta.
  const endedAgo = s?.turnEndedMs ? Date.now() - s.turnEndedMs : Infinity;
  const ttl = s?.status === "done" ? DONE_VISIBLE_MS : s?.status === "error" ? ERROR_VISIBLE_MS : 0;
  const visible = s && (s.status === "working" || s.status === "waiting" || endedAgo < ttl);
  useEffect(() => {
    if (!ttl || endedAgo >= ttl) return;
    const t = window.setTimeout(() => force((n) => n + 1), ttl - endedAgo + 50);
    return () => window.clearTimeout(t);
  }, [ttl, endedAgo]);

  const priority = !s ? 0 : s.status === "waiting" ? 30 : s.status === "error" ? 25 : s.status === "working" ? 20 : 15;
  useActivity(visible ? { id: "agent", priority, Compact: AgentCompact, Badge: AgentBadge, tab: "agent" } : null);
  return null;
}

// ---------------------------------------------------------------- utilidades

function useTicker(active: boolean) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now()); // sin esto, el primer render usa la hora del montaje
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, [active]);
  return active ? Date.now() : now;
}

function fmtDuration(ms: number) {
  const t = Math.max(0, Math.round(ms / 1000));
  if (t < 60) return `${t}s`;
  const m = Math.floor(t / 60);
  if (m < 60) return `${m}m ${(t % 60).toString().padStart(2, "0")}s`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ${(m % 60).toString().padStart(2, "0")}m`;
  return `${Math.floor(h / 24)}d ${h % 24}h`;
}

function fmtTokens(n: number | null) {
  if (n == null) return "—";
  return n >= 1000 ? `${(n / 1000).toFixed(n >= 100_000 ? 0 : 1)}k` : `${n}`;
}

function fmtReset(epochS: number | null) {
  if (!epochS) return "";
  const ms = epochS * 1000 - Date.now();
  if (ms <= 0) return "";
  return `reinicia en ${fmtDuration(ms).replace(/ \d+s$/, "")}`;
}

function turnTime(s: AgentSession, now: number) {
  if (!s.turnStartedMs) return null;
  return (s.turnEndedMs ?? now) - s.turnStartedMs;
}

const STATUS_TEXT: Record<Status, string> = {
  idle: "En espera",
  working: "Trabajando",
  waiting: "Necesita tu permiso",
  done: "Listo",
  error: "Error",
};
const STATUS_COLOR: Record<Status, string> = {
  idle: "rgba(255,255,255,0.5)",
  working: CLAUDE,
  waiting: "#ffb340",
  done: "#30d158",
  error: "#ff453a",
};

function StatusIcon({ status, size }: { status: Status; size: number }) {
  const color = STATUS_COLOR[status];
  if (status === "done") return <Check size={size} color={color} strokeWidth={3} />;
  if (status === "error") return <AlertTriangle size={size} color={color} />;
  if (status === "waiting") return <Hand size={size} color={color} className="agent-pulse" />;
  return <Sparkle size={size} color={color} fill={color} className={status === "working" ? "agent-spin" : ""} />;
}

// ---------------------------------------------------------------- Píldora

function AgentCompact() {
  const s = useStore(agentStore)[0];
  const now = useTicker(s?.status === "working");
  if (!s) return null;
  const t = turnTime(s, now);
  const detail = s.status === "working" ? s.lastAction : s.status === "waiting" ? s.project : s.project;
  return (
    <span className="agent-compact">
      <StatusIcon status={s.status} size={15} />
      <span className="agent-compact-text">
        <span style={{ color: STATUS_COLOR[s.status] }}>{STATUS_TEXT[s.status]}</span>
        {detail && <span className="muted"> · {detail}</span>}
      </span>
      {t != null && s.status !== "waiting" && <span className="agent-time">{fmtDuration(t)}</span>}
    </span>
  );
}

function AgentBadge() {
  const s = useStore(agentStore)[0];
  return s ? <StatusIcon status={s.status} size={14} /> : null;
}

// ---------------------------------------------------------------- Panel

function Meter({ label, pct, sub }: { label: string; pct: number | null; sub?: string }) {
  const p = pct ?? 0;
  const color = p >= 90 ? "#ff453a" : p >= 70 ? "#ffb340" : CLAUDE;
  return (
    <div className="agent-meter">
      <div className="agent-meter-head">
        <span className="label">{label}</span>
        <span className="agent-meter-val">{pct == null ? "—" : `${Math.round(p)}%`}</span>
      </div>
      <div className="agent-bar">
        <div style={{ width: `${Math.min(100, p)}%`, background: color } as CSSProperties} />
      </div>
      {sub && <span className="agent-meter-sub">{sub}</span>}
    </div>
  );
}

export function AgentPanel() {
  const sessions = useStore(agentStore);
  const s = sessions[0];
  const now = useTicker(s?.status === "working");

  if (!s) {
    return (
      <div className="agent-empty">
        <Sparkle size={26} color={CLAUDE} fill={CLAUDE} />
        <p className="muted">Sin sesiones de Claude Code. Al lanzar una, su estado aparecerá aquí.</p>
      </div>
    );
  }

  const u = s.usage;
  const t = turnTime(s, now);
  return (
    <div className="agent-panel">
      <div className="agent-head">
        <StatusIcon status={s.status} size={18} />
        <div className="agent-title">
          <strong>{s.project || "Claude Code"}</strong>
          <span className="muted">
            {STATUS_TEXT[s.status]}
            {t != null ? ` · ${fmtDuration(t)}` : ""}
            {s.toolCount > 0 ? ` · ${s.toolCount} acciones` : ""}
          </span>
        </div>
        {u.model && <span className="agent-chip">{u.model}</span>}
        {sessions.length > 1 && <span className="agent-chip">+{sessions.length - 1}</span>}
      </div>

      <div className="agent-line" title={s.message ?? undefined}>
        {s.status === "working" && s.lastAction ? <code>{s.lastAction}</code> : <span className="muted">{s.message ?? "—"}</span>}
      </div>

      {s.files.length > 0 && (
        <div className="agent-files">
          {s.files.slice(0, 4).map((f) => (
            <span key={f} className="agent-file">
              <FileCode2 size={11} /> {f}
            </span>
          ))}
          {s.files.length > 4 && <span className="muted">+{s.files.length - 4}</span>}
        </div>
      )}

      <div className="agent-meters">
        <Meter label="Contexto" pct={u.contextPct} sub={`${fmtTokens(u.inputTokens)} / ${fmtTokens(u.contextSize)}`} />
        <Meter label="5 horas" pct={u.fiveHourPct} sub={fmtReset(u.fiveHourResets)} />
        <Meter label="Semana" pct={u.sevenDayPct} sub={fmtReset(u.sevenDayResets)} />
      </div>

      <div className="agent-foot muted">
        {u.costUsd != null && <span>Sesión ${u.costUsd.toFixed(2)}</span>}
        {u.linesAdded != null && (
          <span>
            <span style={{ color: "#30d158" }}>+{u.linesAdded}</span> <span style={{ color: "#ff453a" }}>−{u.linesRemoved ?? 0}</span>
          </span>
        )}
        {u.outputTokens != null && <span>salida {fmtTokens(u.outputTokens)}</span>}
      </div>
    </div>
  );
}
