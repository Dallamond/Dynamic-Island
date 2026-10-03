// Módulo de agentes (Claude Code, Codex e IA local): estado del turno en la píldora y detalle en el panel.
// Con varios agentes activos a la vez, la píldora alta enseña una fila por agente.
import { AlertTriangle, Check, Cpu, FileCode2, Hand, Hexagon, Sparkle } from "lucide-react";
import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useActivity } from "../../core/activities";
import { useTauriEvent } from "../../core/ipc";
import { createStore, useStore } from "../../core/store";

type Status = "idle" | "working" | "waiting" | "done" | "error";
type Kind = "claude" | "codex" | "local";

interface Usage {
  model: string | null;
  contextPct: number | null;
  contextSize: number | null;
  inputTokens: number | null;
  outputTokens: number | null;
  costUsd: number | null;
  linesAdded: number | null;
  linesRemoved: number | null;
  limits: Limit[];
  gpuPct: number | null;
}

interface Limit {
  label: string;
  short: string;
  pct: number | null;
  resets: number | null;
}

interface AgentSession {
  id: string;
  kind: Kind;
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
/** Sesiones que se enseñan en la píldora, ya ordenadas (las calcula el Host). */
const shownStore = createStore<AgentSession[]>([]);

/** "Listo" y "error" se quedan en la píldora un rato y luego desaparecen. */
const DONE_VISIBLE_MS = 12_000;
const ERROR_VISIBLE_MS = 30_000;
/** Filas como máximo en la píldora alta. */
const MAX_ROWS = 3;

const COLOR: Record<Kind, string> = { claude: "#d97757", codex: "#8fb4ff", local: "#b48cff" };
const ICON = { claude: Sparkle, codex: Hexagon, local: Cpu };
const agentColor = (s: AgentSession) => COLOR[s.kind] ?? COLOR.claude;
const shortName = (s: AgentSession) => (s.kind === "claude" ? "Claude" : s.agent);

// ---------------------------------------------------------------- Host

function ttlOf(s: AgentSession) {
  return s.status === "done" ? DONE_VISIBLE_MS : s.status === "error" ? ERROR_VISIBLE_MS : 0;
}

const RANK: Record<Status, number> = { waiting: 0, error: 1, working: 2, done: 3, idle: 4 };

/** Visibles en la píldora: trabajando, esperando o terminados hace poco. Primero lo más urgente. */
function visibleSessions(list: AgentSession[], now: number) {
  return list
    .filter((s) => s.status === "working" || s.status === "waiting" || (s.turnEndedMs != null && now - s.turnEndedMs < ttlOf(s)))
    .sort((a, b) => RANK[a.status] - RANK[b.status] || b.lastEventMs - a.lastEventMs);
}

/** Alto de la píldora alta: detalle de 3 filas con un agente, una fila por agente con varios. */
function tallHeight(n: number) {
  return n <= 1 ? 88 : 24 + n * 20 + (n - 1) * 5;
}

export function AgentHost() {
  useTauriEvent<AgentSession[]>("agent://state", agentStore.set);
  useEffect(() => {
    invoke<AgentSession[]>("agent_state").then(agentStore.set);
  }, []);

  const sessions = useStore(agentStore);
  const [tick, force] = useState(0);
  const now = Date.now();
  const shown = useMemo(() => visibleSessions(sessions, now), [sessions, tick]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => shownStore.set(shown), [shown]);

  // Caducidad de "listo"/"error": un único timeout para el próximo que caduque.
  const nextExpiry = Math.min(...shown.filter((s) => ttlOf(s) && s.turnEndedMs).map((s) => s.turnEndedMs! + ttlOf(s)));
  useEffect(() => {
    if (!Number.isFinite(nextExpiry)) return;
    const t = window.setTimeout(() => force((n) => n + 1), Math.max(0, nextExpiry - Date.now()) + 50);
    return () => window.clearTimeout(t);
  }, [nextExpiry]);

  const top = shown[0];
  const priority = !top ? 0 : top.status === "waiting" ? 30 : top.status === "error" ? 25 : top.status === "working" ? 20 : 15;
  const rows = Math.min(shown.length, MAX_ROWS);
  useActivity(
    top ? { id: "agent", priority, Compact: AgentCompact, Tall: AgentTall, tallHeight: tallHeight(rows), Badge: AgentBadge, tab: "agent" } : null,
  );
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
const statusText = (s: AgentSession) =>
  s.kind === "local" ? (s.status === "working" ? "Generando" : s.status === "idle" ? "Modelo cargado" : STATUS_TEXT[s.status]) : STATUS_TEXT[s.status];

const STATUS_COLOR: Record<Exclude<Status, "working">, string> = {
  idle: "rgba(255,255,255,0.5)",
  waiting: "#ffb340",
  done: "#30d158",
  error: "#ff453a",
};

const statusColor = (s: AgentSession) => (s.status === "working" ? agentColor(s) : STATUS_COLOR[s.status]);

function StatusIcon({ s, size }: { s: AgentSession; size: number }) {
  const color = statusColor(s);
  if (s.status === "done") return <Check size={size} color={color} strokeWidth={3} />;
  if (s.status === "error") return <AlertTriangle size={size} color={color} />;
  if (s.status === "waiting") return <Hand size={size} color={color} className="agent-pulse" />;
  const Icon = ICON[s.kind] ?? Sparkle;
  const filled = s.kind === "claude";
  return <Icon size={size} color={color} fill={filled ? color : "none"} strokeWidth={filled ? 2 : 2.4} className={s.status === "working" ? "agent-spin" : ""} />;
}

/** Medidor principal de la fila: GPU para la IA local, contexto para el resto. */
function mainMetric(s: AgentSession): { label: string; pct: number | null } {
  return s.kind === "local" ? { label: "GPU", pct: s.usage.gpuPct } : { label: "Ctx", pct: s.usage.contextPct };
}

const anyWorking = (list: AgentSession[]) => list.some((s) => s.status === "working");

// ---------------------------------------------------------------- Píldora

function AgentCompact() {
  const shown = useStore(shownStore);
  const s = shown[0];
  const now = useTicker(s?.status === "working");
  if (!s) return null;
  const t = turnTime(s, now);
  const detail = s.status === "working" && s.kind !== "local" ? s.lastAction : s.project;
  return (
    <span className="agent-compact">
      <StatusIcon s={s} size={15} />
      <span className="agent-compact-text">
        <span style={{ color: statusColor(s) }}>{statusText(s)}</span>
        {detail && <span className="muted"> · {detail}</span>}
      </span>
      {shown.length > 1 && <span className="agent-more">+{shown.length - 1}</span>}
      {t != null && s.status !== "waiting" && <span className="agent-time">{fmtDuration(t)}</span>}
    </span>
  );
}

/** Píldora alta: detalle de un agente o una fila por agente si hay varios. */
function AgentTall() {
  const shown = useStore(shownStore);
  const now = useTicker(anyWorking(shown));
  if (!shown.length) return null;
  if (shown.length === 1) return <AgentDetail s={shown[0]} now={now} />;
  return (
    <div className="agent-rows">
      {shown.slice(0, MAX_ROWS).map((s) => (
        <AgentRow key={s.id} s={s} now={now} />
      ))}
    </div>
  );
}

function AgentDetail({ s, now }: { s: AgentSession; now: number }) {
  const t = turnTime(s, now);
  const u = s.usage;
  const line =
    s.status === "working" && s.lastAction ? (
      <code>{s.lastAction}</code>
    ) : (
      <span className="muted">{s.message ?? (s.files.length ? s.files.slice(0, 3).join(" · ") : "—")}</span>
    );
  const m = mainMetric(s);
  return (
    <div className="agent-tall">
      <div className="agent-tall-head">
        <StatusIcon s={s} size={15} />
        <span className="agent-compact-text">
          <span style={{ color: statusColor(s), fontWeight: 650 }}>{statusText(s)}</span>
          <span className="muted">
            {" "}
            · {shortName(s)}
            {s.project ? ` · ${s.project}` : ""}
          </span>
        </span>
        {t != null && s.status !== "waiting" && <span className="agent-time">{fmtDuration(t)}</span>}
      </div>
      <div className="agent-line agent-tall-line" title={s.message ?? undefined}>
        {line}
      </div>
      <div className="agent-tall-meters">
        <MiniMeter label={m.label} pct={m.pct} color={agentColor(s)} />
        {u.limits.slice(0, 2).map((l) => (
          <MiniMeter key={l.short} label={l.short} pct={l.pct} color={agentColor(s)} />
        ))}
      </div>
    </div>
  );
}

/** Una fila compacta por agente: icono, nombre y proyecto, acción, medidor y tiempo. */
function AgentRow({ s, now }: { s: AgentSession; now: number }) {
  const t = turnTime(s, now);
  const m = mainMetric(s);
  const action = s.status === "working" ? (s.lastAction ?? statusText(s)) : statusText(s);
  return (
    <div className="agent-row">
      <StatusIcon s={s} size={13} />
      <span className="agent-row-name">
        <span style={{ color: agentColor(s), fontWeight: 650 }}>{shortName(s)}</span>
        {s.project && <span className="muted"> · {s.project}</span>}
      </span>
      <span className="agent-row-action" style={s.status === "working" ? undefined : { color: statusColor(s) }}>
        {action}
      </span>
      <span className="agent-row-metric">
        <span className="muted">{m.label}</span> {m.pct == null ? "—" : `${Math.round(m.pct)}%`}
      </span>
      <span className="agent-time">{t != null && s.status !== "waiting" ? fmtDuration(t) : ""}</span>
    </div>
  );
}

function meterColor(p: number, base: string) {
  return p >= 90 ? "#ff453a" : p >= 70 ? "#ffb340" : base;
}

function MiniMeter({ label, pct, color }: { label: string; pct: number | null; color: string }) {
  const p = pct ?? 0;
  return (
    <span className="agent-mini">
      <span className="muted">{label}</span>
      <span className="agent-bar">
        <span style={{ width: `${Math.min(100, p)}%`, background: meterColor(p, color) }} />
      </span>
      <span className="agent-mini-val">{pct == null ? "—" : `${Math.round(p)}%`}</span>
    </span>
  );
}

function AgentBadge() {
  const s = useStore(shownStore)[0];
  return s ? <StatusIcon s={s} size={14} /> : null;
}

// ---------------------------------------------------------------- Panel

function Meter({ label, pct, sub, base }: { label: string; pct: number | null; sub?: string; base: string }) {
  const p = pct ?? 0;
  const color = meterColor(p, base);
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
  const shown = useStore(shownStore);
  // Primero los activos (en el orden de la píldora) y después el resto.
  const ordered = useMemo(() => [...shown, ...sessions.filter((s) => !shown.some((x) => x.id === s.id))], [sessions, shown]);
  const [selected, setSelected] = useState<string | null>(null);
  const s = ordered.find((x) => x.id === selected) ?? ordered[0];
  const now = useTicker(s?.status === "working");

  if (!s) {
    return (
      <div className="agent-empty">
        <Sparkle size={26} color={COLOR.claude} fill={COLOR.claude} />
        <p className="muted">Sin sesiones de Claude Code, Codex ni IA local. Al lanzar una, su estado aparecerá aquí.</p>
      </div>
    );
  }

  const u = s.usage;
  const t = turnTime(s, now);
  const color = agentColor(s);
  return (
    <div className="agent-panel">
      {ordered.length > 1 && (
        <div className="agent-switch">
          {ordered.map((x) => (
            <button key={x.id} className={`agent-switch-btn${x.id === s.id ? " on" : ""}`} onClick={() => setSelected(x.id)} title={x.project}>
              <StatusIcon s={x} size={12} />
              {shortName(x)}
            </button>
          ))}
        </div>
      )}

      <div className="agent-head">
        <StatusIcon s={s} size={18} />
        <div className="agent-title">
          <strong>{s.project || s.agent}</strong>
          <span className="muted">
            {statusText(s)}
            {t != null ? ` · ${fmtDuration(t)}` : ""}
            {s.toolCount > 0 ? ` · ${s.toolCount} acciones` : ""}
          </span>
        </div>
        <span className="agent-chip" style={{ color }}>
          {s.agent}
        </span>
        {u.model && u.model !== s.project && <span className="agent-chip">{u.model}</span>}
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
        {s.kind === "local" ? (
          <>
            <Meter label="GPU" pct={u.gpuPct} sub={`generando a partir del 60 %`} base={color} />
            <Meter label="Contexto" pct={null} sub={`${fmtTokens(u.contextSize)} cargado`} base={color} />
          </>
        ) : (
          <Meter label="Contexto" pct={u.contextPct} sub={`${fmtTokens(u.inputTokens)} / ${fmtTokens(u.contextSize)}`} base={color} />
        )}
        {u.limits.slice(0, 2).map((l) => (
          <Meter key={l.label} label={l.label} pct={l.pct} sub={fmtReset(l.resets)} base={color} />
        ))}
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
