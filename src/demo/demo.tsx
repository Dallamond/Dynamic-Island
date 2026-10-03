// Demo para capturas: la app real con el backend de Rust simulado (mocks de Tauri) y datos de ejemplo.
// Uso: `npm run dev` y abrir http://localhost:1420/demo.html?scene=<escena>. Ver scripts/capture-docs.mjs.
import ReactDOM from "react-dom/client";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { MotionGlobalConfig } from "motion/react";
import type { Settings } from "../core/types";

// Chrome sin ventana no completa las animaciones: en las capturas terminan al instante.
MotionGlobalConfig.instantAnimations = true;

const params = new URLSearchParams(location.search);
const scene = params.get("scene") ?? "clock";
const now = Date.now();
const MIN = 60_000;

// ---------------------------------------------------------------- datos de ejemplo

const settings: Settings = {
  appearance: {
    background: "#000000",
    foreground: "#ffffff",
    accent: "#0a84ff",
    opacity: 1,
    scale: 1,
    margin: 6,
    showSeconds: false,
    // En la escena "peek" la isla no debe llegar a expandirse.
    expandDelayMs: scene === "peek" ? 600_000 : 450,
    collapseDelayMs: 350,
  },
  modules: { media: true, agent: true, codex: true, localAi: true, system: true, timer: true, calc: true, notes: true },
  shortcuts: { toggleExpand: "Ctrl+Alt+I", nextMonitor: "Ctrl+Alt+M", toggleHidden: "Ctrl+Alt+H" },
  localAi: { endpoint: "http://127.0.0.1:1234", name: "Bionic" },
  autostart: true,
  multiMonitor: true,
  minimalInFullscreen: true,
  minimalOnActiveMonitor: false,
  activityDetail: "primary",
  activeMonitor: "DEMO1",
  positions: {},
  monitorFocus: { DEMO2: "timer", DEMO3: "agent" },
};

const monitors = [
  { name: "DEMO2", x: -1080, y: -294, width: 1080, height: 1920, scale: 1, primary: false },
  { name: "DEMO1", x: 0, y: 0, width: 2560, height: 1440, scale: 1, primary: true },
  { name: "DEMO3", x: 2560, y: 154, width: 1920, height: 1080, scale: 1, primary: false },
];

const usage = (o: Record<string, unknown> = {}) => ({
  model: null,
  contextPct: null,
  contextSize: null,
  inputTokens: null,
  outputTokens: null,
  costUsd: null,
  linesAdded: null,
  linesRemoved: null,
  limits: [],
  gpuPct: null,
  ...o,
});

const claude = {
  id: "c1",
  kind: "claude",
  agent: "Claude Code",
  project: "mi-web",
  cwd: "C:/proyectos/mi-web",
  status: "working",
  lastAction: "Edit Header.tsx",
  message: "Añade un modo oscuro a la cabecera",
  files: ["Header.tsx", "theme.css", "App.tsx"],
  toolCount: 14,
  turnStartedMs: now - 2 * MIN - 14_000,
  turnEndedMs: null,
  lastEventMs: now - 2_000,
  usage: usage({
    model: "Opus",
    contextPct: 42,
    contextSize: 200_000,
    inputTokens: 84_000,
    outputTokens: 12_400,
    costUsd: 1.84,
    linesAdded: 128,
    linesRemoved: 31,
    limits: [
      { label: "5 horas", short: "5h", pct: 23, resets: Math.floor(now / 1000) + 2 * 3600 + 40 * 60 },
      { label: "Semana", short: "Sem", pct: 9, resets: Math.floor(now / 1000) + 4 * 86400 },
    ],
  }),
};

const codex = {
  ...claude,
  id: "x1",
  kind: "codex",
  agent: "Codex",
  project: "api-tienda",
  lastAction: "Shell npm test",
  files: ["orders.ts"],
  toolCount: 6,
  turnStartedMs: now - 48_000,
  lastEventMs: now - 3_000,
  usage: usage({
    model: "gpt-6",
    contextPct: 12,
    contextSize: 258_400,
    inputTokens: 31_000,
    limits: [{ label: "Semana", short: "Sem", pct: 17, resets: Math.floor(now / 1000) + 3 * 86400 }],
  }),
};

const local = {
  ...claude,
  id: "local:http://127.0.0.1:1234",
  kind: "local",
  agent: "Bionic",
  project: "qwen3.5-9b",
  lastAction: "Generando",
  message: null,
  files: [],
  toolCount: 0,
  turnStartedMs: now - 12_000,
  lastEventMs: now - 4_000,
  usage: usage({ model: "qwen3.5-9b", contextSize: 8192, gpuPct: 93 }),
};

const waiting = { ...claude, status: "waiting", message: "Claude necesita permiso para usar Bash", lastAction: "Bash npm run build" };

const agentsByScene: Record<string, unknown[]> = {
  "agent-tall": [claude],
  "agent-waiting": [waiting],
  "agent-expanded": [claude, codex],
  "agents-multi": [claude, codex, local],
  "agents-timer": [claude],
  "home-expanded": [claude],
  "focus-timer": [claude],
};

const pomodoroRunning = ["agents-multi", "agents-timer", "timer-expanded", "home-expanded", "focus-timer"].includes(scene);
const timers = {
  stopwatch: { running: false, accumulatedMs: 0, startedAtMs: null },
  countdown: { durationMs: 10 * MIN, running: false, remainingMs: 10 * MIN, endsAtMs: null },
  pomodoro: {
    phase: "work",
    running: pomodoroRunning,
    remainingMs: 18 * MIN + 42_000,
    endsAtMs: pomodoroRunning ? now + 18 * MIN + 42_000 : null,
    completed: 3,
    workMin: 25,
    shortMin: 5,
    longMin: 15,
  },
};

// Carátula de ejemplo (SVG propio, sin derechos de terceros).
const cover =
  "data:image/svg+xml;base64," +
  btoa(
    `<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff5f6d"/><stop offset="1" stop-color="#7b2ff7"/></linearGradient></defs><rect width="300" height="300" fill="url(#g)"/><circle cx="210" cy="95" r="48" fill="#ffd166" opacity=".9"/><path d="M0 230 Q75 170 150 220 T300 200 V300 H0Z" fill="#1b1035" opacity=".75"/></svg>`,
  );

const mediaPlaying = ["media", "media-expanded", "peek", "home-expanded", "agents-timer"].includes(scene);
const media = {
  appId: "Spotify.exe",
  appName: "Spotify",
  title: "Luces de la ciudad",
  artist: "Nébula Sur",
  album: "Horizonte",
  playing: true,
  positionMs: 82_000,
  durationMs: 214_000,
  updatedAtMs: now,
  canPrev: true,
  canNext: true,
  canSeek: true,
  thumbnail: cover,
};

const system = {
  cpu: 23,
  cpuName: "AMD Ryzen 5 3600",
  cores: 12,
  freqMhz: 4100,
  memUsed: 11.4 * 1024 ** 3,
  memTotal: 32 * 1024 ** 3,
  swapUsed: 1.1 * 1024 ** 3,
  swapTotal: 8 * 1024 ** 3,
  gpu: { name: "RTX 3060", usage: 37, memUsed: 5.2 * 1024 ** 3, memTotal: 12 * 1024 ** 3, temp: 58, powerW: 71, fan: 41 },
  top: [
    { name: "chrome.exe", cpu: 8.2, mem: 2.1 * 1024 ** 3 },
    { name: "Code.exe", cpu: 4.9, mem: 1.4 * 1024 ** 3 },
    { name: "Spotify.exe", cpu: 1.3, mem: 0.4 * 1024 ** 3 },
  ],
};

const notes = [
  { id: "1", text: "Comprar filamento PLA negro", done: false, createdMs: now - 3 * 86400_000 },
  { id: "2", text: "Revisar el PR del modo oscuro", done: true, createdMs: now - 86400_000 },
  { id: "3", text: "Llamar al taller antes de las 18:00", done: false, createdMs: now - 3600_000 },
];

const audio = {
  volume: 0.64,
  muted: false,
  devices: [
    { id: "a", name: "Altavoces (Realtek)", isDefault: true },
    { id: "b", name: "Auriculares USB", isDefault: false },
  ],
};

// ---------------------------------------------------------------- backend simulado

const isSettings = scene === "settings";
mockWindows(isSettings ? "settings" : "main");
mockIPC(
  (cmd) => {
    switch (cmd) {
      case "get_settings":
        return settings;
      case "get_layout":
        return { edge: "top", monitor: scene === "focus-timer" ? "DEMO2" : "DEMO1" };
      case "list_monitors":
        return monitors;
      case "agent_state":
        return agentsByScene[scene] ?? [];
      case "timer_state":
      case "timer_action":
        return timers;
      case "audio_state":
        return audio;
      case "notes_get":
        return notes;
      case "media_refresh":
        if (mediaPlaying) setTimeout(() => emit("media://state", media), 0);
        return null;
      case "system_watch":
        setTimeout(() => emit("system://stats", system), 0);
        return null;
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

// ---------------------------------------------------------------- escena

const css = document.createElement("style");
css.textContent = isSettings
  ? ""
  : `html, body { background: radial-gradient(120% 140% at 50% 0%, #3a4a6b 0%, #1d2333 45%, #0d1018 100%) !important; }
     .canvas { margin: 0 auto; }`;
document.head.appendChild(css);

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const click = (sel: string) => (document.querySelector(sel) as HTMLElement | null)?.click();

async function boot() {
  if (isSettings) {
    await import("../settings/settings.css");
    const { SettingsApp } = await import("../settings/SettingsApp");
    root.render(<SettingsApp />);
    return;
  }
  await import("../island/island.css");
  const { IslandApp } = await import("../island/IslandApp");
  root.render(<IslandApp />);
  await wait(400);
  if (mediaPlaying) await emit("media://state", media);

  const tab: Record<string, string> = {
    "media-expanded": "Música",
    "agent-expanded": "Agentes",
    "timer-expanded": "Temporizadores",
    "system-expanded": "Sistema",
    "calc-expanded": "Calculadora",
    "notes-expanded": "Notas",
    "home-expanded": "Inicio",
  };
  if (scene === "peek") await emit("island://hover", true);
  if (scene === "minimal") await emit("island://minimal", true);
  if (scene in tab) {
    await emit("island://toggle", null);
    await wait(500);
    click(`button.tab[title="${tab[scene]}"]`);
    if (scene === "calc-expanded") {
      await wait(200);
      for (const k of ["1", "2", "8", "×", "1", "5"]) {
        [...document.querySelectorAll<HTMLButtonElement>(".calc-key")].find((b) => b.textContent === k)?.click();
      }
    }
  }
  // Señal para el script de capturas.
  await wait(900);
  document.body.dataset.ready = "1";
}

boot();
