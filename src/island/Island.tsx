// La isla: una píldora negra dentro de un lienzo transparente.
// Solo la píldora es "sólida" para el ratón: su rectángulo se envía a Rust en cada cambio.
import { AnimatePresence, motion } from "motion/react";
import { Settings2 } from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { useActivities } from "../core/activities";
import { ErrorBoundary } from "../core/ErrorBoundary";
import { ipc, useTauriEvent } from "../core/ipc";
import type { Edge, Layout, Settings } from "../core/types";
import { CompactClock } from "../modules/clock/Clock";
import { MODULES } from "../modules/registry";
import { useIslandMode, type Mode } from "./useIslandMode";

const SIZES: Record<Mode, { w: number; h: number; r: number }> = {
  collapsed: { w: 168, h: 34, r: 17 },
  peek: { w: 300, h: 46, r: 23 },
  expanded: { w: 448, h: 252, r: 30 },
};
/** Ancho cerrado cuando hay una actividad (música, agente...) en la píldora. */
const COLLAPSED_ACTIVE_W = 248;

const spring = { type: "spring", stiffness: 420, damping: 34, mass: 0.9 } as const;

const NO_DRAG = "button, input, textarea, select, a, [data-nodrag]";

export function Island({ settings }: { settings: Settings }) {
  const { appearance: ap } = settings;
  const { mode, dragging, onClick } = useIslandMode(ap.expandDelayMs, ap.collapseDelayMs);
  const [edge, setEdge] = useState<Edge>("top");
  const pill = useRef<HTMLDivElement>(null);
  const activities = useActivities();
  const primary = activities[0];
  const secondary = activities[1];

  const modules = useMemo(() => MODULES.filter((m) => m.enabled(settings)), [settings]);
  const [tab, setTab] = useState("home");
  const activeTab = modules.find((m) => m.id === tab) ?? modules[0];

  // Al expandir, saltar a la pestaña de la actividad principal (p. ej. música).
  useEffect(() => {
    if (mode === "expanded" && primary?.tab && modules.some((m) => m.id === primary.tab)) setTab(primary.tab);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  useEffect(() => {
    ipc.getLayout().then((l) => setEdge(l.edge));
  }, []);
  useTauriEvent<Layout>("island://layout", (l) => setEdge(l.edge));

  // Rectángulo de la píldora → Rust (hover y click-through). Throttle a un frame.
  useLayoutEffect(() => {
    const el = pill.current;
    if (!el) return;
    let raf = 0;
    const report = () => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const r = el.getBoundingClientRect();
        ipc.setHitRect(r.x, r.y, r.width, r.height);
      });
    };
    const ro = new ResizeObserver(report);
    ro.observe(el);
    report();
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [edge]);

  const size = SIZES[mode];
  const w = mode === "collapsed" && primary ? COLLAPSED_ACTIVE_W : size.w;

  const onPointerDown = (e: PointerEvent) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest(NO_DRAG)) return;
    ipc.dragBegin();
  };

  const style = {
    "--bg": ap.background,
    "--fg": ap.foreground,
    "--accent": ap.accent,
    opacity: ap.opacity,
  } as CSSProperties;

  return (
    <div className={`canvas edge-${edge}`} style={{ zoom: ap.scale }}>
      <motion.div
        ref={pill}
        className={`pill mode-${mode}${dragging ? " dragging" : ""}`}
        style={style}
        initial={false}
        animate={{ width: w, height: size.h, borderRadius: size.r }}
        transition={spring}
        onPointerDown={onPointerDown}
        onClick={mode === "expanded" ? undefined : onClick}
      >
        <AnimatePresence mode="popLayout" initial={false}>
          {mode === "expanded" ? (
            <motion.div
              key="expanded"
              className="expanded"
              initial={{ opacity: 0, scale: 0.96, filter: "blur(6px)" }}
              animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, scale: 0.96, filter: "blur(6px)", transition: { duration: 0.12 } }}
              transition={{ duration: 0.22, delay: 0.06 }}
            >
              <header className="tabs">
                {modules.map((m) => (
                  <button
                    key={m.id}
                    className={`tab${m.id === activeTab?.id ? " active" : ""}`}
                    title={m.label}
                    onClick={() => setTab(m.id)}
                  >
                    <m.icon size={15} strokeWidth={2.2} />
                  </button>
                ))}
                <span className="tabs-spacer" />
                <CompactClock seconds={false} />
                <button className="tab" title="Ajustes" onClick={() => ipc.openSettings()}>
                  <Settings2 size={15} strokeWidth={2.2} />
                </button>
              </header>
              <section className="panel">
                <ErrorBoundary resetKey={activeTab?.id} fallback={<p className="muted">Este módulo ha fallado.</p>}>
                  {activeTab && <activeTab.Panel />}
                </ErrorBoundary>
              </section>
            </motion.div>
          ) : (
            <motion.div
              key={`compact-${primary?.id ?? "clock"}`}
              className={`compact${mode === "peek" ? " peek" : ""}`}
              initial={{ opacity: 0, filter: "blur(4px)" }}
              animate={{ opacity: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, filter: "blur(4px)", transition: { duration: 0.1 } }}
              transition={{ duration: 0.18 }}
            >
              <ErrorBoundary resetKey={primary?.id} fallback={<CompactClock seconds={ap.showSeconds} />}>
                {primary ? <primary.Compact /> : <CompactClock seconds={ap.showSeconds} />}
              </ErrorBoundary>
              {secondary?.Badge && (
                <span className="compact-badge">
                  <secondary.Badge />
                </span>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
    </div>
  );
}
