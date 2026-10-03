// Genera las capturas de docs/img con Chrome sin ventana sobre la demo (datos de ejemplo, sin datos reales).
// Uso: con `npm run dev` arrancado, `node scripts/capture-docs.mjs [escena...]`.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const CHROME = [
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find(existsSync);
if (!CHROME) throw new Error("No se encontró Chrome ni Edge");

const BASE = process.env.DEMO_URL ?? "http://localhost:1420/demo.html";
const OUT = resolve("docs/img");
mkdirSync(OUT, { recursive: true });

// escena → [ancho, alto] de la captura
const SCENES = {
  clock: [480, 64],
  peek: [480, 70],
  media: [480, 64],
  "agent-tall": [480, 112],
  "agent-waiting": [480, 112],
  "agents-timer": [480, 112],
  "agents-multi": [480, 112],
  "focus-timer": [480, 64],
  minimal: [480, 40],
  "home-expanded": [480, 300],
  "media-expanded": [480, 300],
  "agent-expanded": [480, 300],
  "timer-expanded": [480, 300],
  "system-expanded": [480, 300],
  "calc-expanded": [480, 300],
  "notes-expanded": [480, 300],
  settings: [600, 1990],
};

const wanted = process.argv.slice(2);
for (const [scene, [w, h]] of Object.entries(SCENES)) {
  if (wanted.length && !wanted.includes(scene)) continue;
  const file = resolve(OUT, `${scene}.png`);
  execFileSync(CHROME, [
    "--headless=new",
    "--disable-gpu",
    "--hide-scrollbars",
    "--force-device-scale-factor=2",
    `--window-size=${w},${h}`,
    "--virtual-time-budget=6000",
    `--screenshot=${file}`,
    `${BASE}?scene=${scene}`,
  ], { stdio: "ignore" });
  console.log("✓", scene);
}
