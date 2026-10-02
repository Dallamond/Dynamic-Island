// Sistema: CPU, RAM y GPU. Rust solo sondea mientras algún componente esté mirando.
import { Cpu, Gpu, MemoryStick } from "lucide-react";
import { useEffect, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTauriEvent } from "../../core/ipc";
import { createStore, useStore } from "../../core/store";

interface GpuStats {
  name: string;
  usage: number;
  memUsed: number;
  memTotal: number;
  temp: number | null;
  powerW: number | null;
  fan: number | null;
}
interface SystemStats {
  cpu: number;
  cpuName: string;
  cores: number;
  freqMhz: number;
  memUsed: number;
  memTotal: number;
  swapUsed: number;
  swapTotal: number;
  gpu: GpuStats | null;
  top: { name: string; cpu: number; mem: number }[];
}

const statsStore = createStore<SystemStats | null>(null);
let watchers = 0;

/** Estadísticas en vivo; arranca el sondeo con el primer observador y lo para con el último. */
export function useSystemStats(): SystemStats | null {
  useTauriEvent<SystemStats>("system://stats", statsStore.set);
  useEffect(() => {
    if (watchers++ === 0) invoke("system_watch", { on: true });
    return () => {
      if (--watchers === 0) invoke("system_watch", { on: false });
    };
  }, []);
  return useStore(statsStore);
}

const gb = (b: number) => (b / 1024 ** 3).toFixed(1);
const pct = (a: number, b: number) => (b > 0 ? (a / b) * 100 : 0);
const level = (p: number) => (p >= 90 ? "#ff453a" : p >= 70 ? "#ffb340" : "var(--accent)");

/** Fila mínima para la vista de inicio. */
export function SystemMini() {
  const s = useSystemStats();
  if (!s) return <div className="sys-mini muted">…</div>;
  return (
    <div className="sys-mini">
      <span>
        <Cpu size={13} /> {Math.round(s.cpu)}%
      </span>
      <span>
        <MemoryStick size={13} /> {Math.round(pct(s.memUsed, s.memTotal))}%
      </span>
      {s.gpu && (
        <span>
          <Gpu size={13} /> {s.gpu.usage}%{s.gpu.temp != null ? ` · ${s.gpu.temp}°` : ""}
        </span>
      )}
    </div>
  );
}

function Ring({ value, label, sub, icon: Icon }: { value: number; label: string; sub: string; icon: typeof Cpu }) {
  const r = 26;
  const c = 2 * Math.PI * r;
  const v = Math.min(100, Math.max(0, value));
  return (
    <div className="sys-ring">
      <svg width="64" height="64" viewBox="0 0 64 64">
        <circle cx="32" cy="32" r={r} stroke="rgba(255,255,255,0.12)" strokeWidth="6" fill="none" />
        <circle
          cx="32"
          cy="32"
          r={r}
          stroke={level(v)}
          strokeWidth="6"
          fill="none"
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - v / 100)}
          transform="rotate(-90 32 32)"
          style={{ transition: "stroke-dashoffset 0.6s" } as CSSProperties}
        />
        <text x="32" y="37" textAnchor="middle" fontSize="15" fontWeight="650" fill="currentColor">
          {Math.round(v)}
        </text>
      </svg>
      <div className="sys-ring-label">
        <Icon size={12} /> {label}
      </div>
      <div className="sys-ring-sub">{sub}</div>
    </div>
  );
}

export function SystemPanel() {
  const s = useSystemStats();
  if (!s) return <div className="sys-loading muted">Midiendo…</div>;
  const g = s.gpu;
  return (
    <div className="sys-panel">
      <div className="sys-rings">
        <Ring icon={Cpu} label="CPU" value={s.cpu} sub={`${s.cores} hilos · ${(s.freqMhz / 1000).toFixed(1)} GHz`} />
        <Ring icon={MemoryStick} label="RAM" value={pct(s.memUsed, s.memTotal)} sub={`${gb(s.memUsed)} / ${gb(s.memTotal)} GB`} />
        {g ? (
          <Ring icon={Gpu} label={g.name || "GPU"} value={g.usage} sub={`${gb(g.memUsed)} / ${gb(g.memTotal)} GB VRAM`} />
        ) : (
          <div className="sys-ring muted">Sin GPU NVIDIA</div>
        )}
      </div>
      {g && (
        <div className="sys-gpu-extra muted">
          {g.temp != null && <span>{g.temp} °C</span>}
          {g.powerW != null && <span>{g.powerW.toFixed(0)} W</span>}
          {g.fan != null && <span>ventilador {g.fan}%</span>}
        </div>
      )}
      <div className="sys-top">
        {s.top.map((p) => (
          <div key={p.name} className="sys-proc">
            <span className="sys-proc-name">{p.name}</span>
            <span>{p.cpu.toFixed(1)}%</span>
            <span className="muted">{(p.mem / 1024 ** 2).toFixed(0)} MB</span>
          </div>
        ))}
      </div>
    </div>
  );
}
