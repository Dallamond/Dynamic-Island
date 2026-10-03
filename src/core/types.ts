// Tipos espejo de src-tauri/src/settings.rs (camelCase por serde).

export type Edge = "top" | "left" | "right";

/** Monitores donde la píldora cerrada crece para dar más detalle de una actividad. */
export type ActivityDetail = "primary" | "all" | "off";

export interface Dock {
  edge: Edge;
  offset: number;
}

export interface Appearance {
  background: string;
  foreground: string;
  accent: string;
  opacity: number;
  scale: number;
  margin: number;
  showSeconds: boolean;
  expandDelayMs: number;
  collapseDelayMs: number;
}

export interface Modules {
  media: boolean;
  agent: boolean;
  system: boolean;
  timer: boolean;
  calc: boolean;
  notes: boolean;
}

export interface Shortcuts {
  toggleExpand: string;
  nextMonitor: string;
  toggleHidden: string;
}

export interface Settings {
  appearance: Appearance;
  modules: Modules;
  shortcuts: Shortcuts;
  autostart: boolean;
  multiMonitor: boolean;
  minimalInFullscreen: boolean;
  minimalOnActiveMonitor: boolean;
  activityDetail: ActivityDetail;
  activeMonitor: string | null;
  positions: Record<string, Dock>;
}

export interface Layout {
  edge: Edge;
  monitor: string;
}

export interface MonitorInfo {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  primary: boolean;
}
