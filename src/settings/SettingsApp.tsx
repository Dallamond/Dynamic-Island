// Ventana de ajustes. Cada cambio se guarda al momento (con un pequeño debounce).
import { useEffect, useRef, useState, type ReactNode } from "react";
import { ipc, useTauriEvent } from "../core/ipc";
import type { ActivityDetail, Appearance, Modules, MonitorInfo, Settings, Shortcuts } from "../core/types";

export function SettingsApp() {
  const [s, setS] = useState<Settings | null>(null);
  const [errors, setErrors] = useState<string[]>([]);
  const saveTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    ipc.getSettings().then(setS);
  }, []);
  // Cambios hechos desde fuera (arrastrar la isla, bandeja, atajos).
  useTauriEvent<Settings>("settings://changed", (next) => {
    if (saveTimer.current === undefined) setS(next);
  });
  useTauriEvent<string[]>("settings://shortcut-errors", setErrors);

  if (!s) return null;

  const update = (next: Settings) => {
    setS(next);
    setErrors([]);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveTimer.current = undefined;
      ipc.saveSettings(next);
    }, 250);
  };
  const ap = <K extends keyof Appearance>(k: K, v: Appearance[K]) => update({ ...s, appearance: { ...s.appearance, [k]: v } });
  const mod = (k: keyof Modules, v: boolean) => update({ ...s, modules: { ...s.modules, [k]: v } });
  const sc = (k: keyof Shortcuts, v: string) => update({ ...s, shortcuts: { ...s.shortcuts, [k]: v } });

  return (
    <main className="settings">
      <h1>Dynamic Island</h1>

      <Section title="Aspecto">
        <Row label="Fondo">
          <input type="color" value={s.appearance.background} onChange={(e) => ap("background", e.target.value)} />
        </Row>
        <Row label="Texto">
          <input type="color" value={s.appearance.foreground} onChange={(e) => ap("foreground", e.target.value)} />
        </Row>
        <Row label="Acento">
          <input type="color" value={s.appearance.accent} onChange={(e) => ap("accent", e.target.value)} />
        </Row>
        <Slider label="Opacidad" min={0.4} max={1} step={0.05} value={s.appearance.opacity} fmt={(v) => `${Math.round(v * 100)} %`} onChange={(v) => ap("opacity", v)} />
        <Slider label="Tamaño" min={0.8} max={1.4} step={0.05} value={s.appearance.scale} fmt={(v) => `${Math.round(v * 100)} %`} onChange={(v) => ap("scale", v)} />
        <Slider label="Separación del borde" min={0} max={40} step={1} value={s.appearance.margin} fmt={(v) => `${v} px`} onChange={(v) => ap("margin", v)} />
        <Slider label="Retardo al expandir" min={0} max={1500} step={50} value={s.appearance.expandDelayMs} fmt={(v) => `${v} ms`} onChange={(v) => ap("expandDelayMs", v)} />
        <Slider label="Retardo al cerrar" min={0} max={1500} step={50} value={s.appearance.collapseDelayMs} fmt={(v) => `${v} ms`} onChange={(v) => ap("collapseDelayMs", v)} />
        <Toggle label="Segundos en el reloj cerrado" value={s.appearance.showSeconds} onChange={(v) => ap("showSeconds", v)} />
        <Row label="Píldora ampliada con Claude Code">
          <select
            className="select"
            value={s.activityDetail}
            onChange={(e) => update({ ...s, activityDetail: e.target.value as ActivityDetail })}
          >
            <option value="primary">Solo monitor principal</option>
            <option value="all">Todos los monitores</option>
            <option value="off">Nunca</option>
          </select>
        </Row>
      </Section>

      <Section title="Posición">
        <MonitorMap active={s.activeMonitor} />
        <p className="hint">Arrastra la isla para moverla: se imanta arriba o a un lateral del monitor donde la sueltes.</p>
        <Toggle label="Una isla en cada monitor" value={s.multiMonitor} onChange={(v) => update({ ...s, multiMonitor: v })} />
        <Toggle label="Mínima con pantalla completa" value={s.minimalInFullscreen} onChange={(v) => update({ ...s, minimalInFullscreen: v })} />
        {s.multiMonitor && (
          <Toggle
            label="Mínima en el monitor que estás usando"
            value={s.minimalOnActiveMonitor}
            onChange={(v) => update({ ...s, minimalOnActiveMonitor: v })}
          />
        )}
        <button className="button" onClick={() => ipc.resetPosition()}>
          Volver arriba al centro del monitor principal
        </button>
      </Section>

      <Section title="Módulos">
        <Toggle label="Música y audio" value={s.modules.media} onChange={(v) => mod("media", v)} />
        <Toggle label="Claude Code" value={s.modules.agent} onChange={(v) => mod("agent", v)} />
        <Toggle label="Sistema (CPU, RAM, GPU)" value={s.modules.system} onChange={(v) => mod("system", v)} />
        <Toggle label="Temporizadores" value={s.modules.timer} onChange={(v) => mod("timer", v)} />
        <Toggle label="Calculadora" value={s.modules.calc} onChange={(v) => mod("calc", v)} />
        <Toggle label="Notas rápidas" value={s.modules.notes} onChange={(v) => mod("notes", v)} />
      </Section>

      <Section title="Atajos globales">
        <ShortcutRow label="Expandir / cerrar" value={s.shortcuts.toggleExpand} onChange={(v) => sc("toggleExpand", v)} />
        <ShortcutRow label="Siguiente monitor" value={s.shortcuts.nextMonitor} onChange={(v) => sc("nextMonitor", v)} />
        <ShortcutRow label="Ocultar / mostrar" value={s.shortcuts.toggleHidden} onChange={(v) => sc("toggleHidden", v)} />
        {errors.length > 0 && <p className="error">No se pudieron registrar: {errors.join(" · ")}</p>}
      </Section>

      <Section title="General">
        <Toggle label="Arrancar con Windows" value={s.autostart} onChange={(v) => update({ ...s, autostart: v })} />
        <button className="button danger" onClick={() => ipc.quit()}>
          Cerrar la isla
        </button>
      </Section>
    </main>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="section">
      <h2>{title}</h2>
      {children}
    </section>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="row">
      <span>{label}</span>
      {children}
    </label>
  );
}

function Toggle({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <Row label={label}>
      <input type="checkbox" className="switch" checked={value} onChange={(e) => onChange(e.target.checked)} />
    </Row>
  );
}

function Slider(p: { label: string; min: number; max: number; step: number; value: number; fmt: (v: number) => string; onChange: (v: number) => void }) {
  return (
    <Row label={p.label}>
      <span className="slider">
        <input type="range" min={p.min} max={p.max} step={p.step} value={p.value} onChange={(e) => p.onChange(Number(e.target.value))} />
        <output>{p.fmt(p.value)}</output>
      </span>
    </Row>
  );
}

/** Graba un atajo pulsándolo. Formato compatible con el parser de tauri-plugin-global-shortcut. */
function ShortcutRow({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  const [recording, setRecording] = useState(false);
  return (
    <Row label={label}>
      <span className="shortcut">
        <button
          className={`kbd${recording ? " recording" : ""}`}
          onClick={() => setRecording(true)}
          onBlur={() => setRecording(false)}
          onKeyDown={(e) => {
            if (!recording) return;
            e.preventDefault();
            if (e.key === "Escape") return setRecording(false);
            const key = codeToKey(e.code);
            if (!key) return; // solo modificadores por ahora
            const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
            if (mods.length === 0) return; // exigir al menos un modificador
            onChange([...mods, key].join("+"));
            setRecording(false);
          }}
        >
          {recording ? "Pulsa la combinación…" : value || "—"}
        </button>
        {value && !recording && (
          <button className="link" onClick={() => onChange("")}>
            quitar
          </button>
        )}
      </span>
    </Row>
  );
}

function codeToKey(code: string): string | null {
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(code)) return null;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code; // Space, F1, ArrowRight, Numpad1...
}

/** Mini-mapa de monitores a escala: clic para mover la isla a ese monitor. */
function MonitorMap({ active }: { active: string | null }) {
  const [mons, setMons] = useState<MonitorInfo[]>([]);
  useEffect(() => {
    ipc.listMonitors().then(setMons);
  }, []);
  if (mons.length === 0) return null;
  const minX = Math.min(...mons.map((m) => m.x));
  const minY = Math.min(...mons.map((m) => m.y));
  const maxX = Math.max(...mons.map((m) => m.x + m.width));
  const maxY = Math.max(...mons.map((m) => m.y + m.height));
  const k = 520 / (maxX - minX);
  const current = active ?? mons.find((m) => m.primary)?.name;
  return (
    <div className="monitor-map" style={{ height: (maxY - minY) * k }}>
      {mons.map((m, i) => (
        <button
          key={m.name}
          className={`monitor${m.name === current ? " active" : ""}`}
          style={{ left: (m.x - minX) * k, top: (m.y - minY) * k, width: m.width * k - 4, height: m.height * k - 4 }}
          onClick={() => ipc.moveToMonitor(m.name)}
          title={m.name}
        >
          <strong>{i + 1}</strong>
          <small>
            {m.width}×{m.height}
            {m.primary ? " · principal" : ""}
          </small>
        </button>
      ))}
    </div>
  );
}
