// Conecta Claude Code con la Dynamic Island (idempotente).
// - Copia el statusLine a ~/.claude/island-statusline.mjs
// - Añade hooks HTTP hacia http://127.0.0.1:47823/hook en ~/.claude/settings.json
// - Configura statusLine solo si no hay uno (o si ya era el de la isla)
// Hace copia de seguridad de settings.json antes de tocarlo.
// Uso: node scripts/install-claude-hooks.mjs [--uninstall]
import { copyFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const URL = "http://127.0.0.1:47823/hook";
const EVENTS = [
  "SessionStart",
  "UserPromptSubmit",
  "PreToolUse",
  "PostToolUse",
  "PostToolUseFailure",
  "Notification",
  "Stop",
  "StopFailure",
  "SessionEnd",
];
const uninstall = process.argv.includes("--uninstall");
const claudeDir = join(homedir(), ".claude");
const settingsPath = join(claudeDir, "settings.json");
const here = dirname(fileURLToPath(import.meta.url));
const statusTarget = join(claudeDir, "island-statusline.mjs");
const statusCommand = "node ~/.claude/island-statusline.mjs";

const raw = existsSync(settingsPath) ? readFileSync(settingsPath, "utf8") : "{}";
const settings = JSON.parse(raw.replace(/^﻿/, ""));
const stamp = new Date().toISOString().replace(/[:.]/g, "-");
writeFileSync(`${settingsPath}.backup-${stamp}`, raw);

settings.hooks ??= {};
const isOurs = (h) => h?.type === "http" && h?.url === URL;
for (const ev of EVENTS) {
  const groups = (settings.hooks[ev] ?? [])
    .map((g) => ({ ...g, hooks: (g.hooks ?? []).filter((h) => !isOurs(h)) }))
    .filter((g) => g.hooks.length > 0);
  if (!uninstall) groups.push({ hooks: [{ type: "http", url: URL, timeout: 2 }] });
  if (groups.length) settings.hooks[ev] = groups;
  else delete settings.hooks[ev];
}
if (Object.keys(settings.hooks).length === 0) delete settings.hooks;

if (uninstall) {
  if (settings.statusLine?.command === statusCommand) delete settings.statusLine;
} else {
  copyFileSync(join(here, "island-statusline.mjs"), statusTarget);
  if (!settings.statusLine || settings.statusLine.command === statusCommand) {
    settings.statusLine = { type: "command", command: statusCommand };
  } else {
    console.log("Ya tienes un statusLine propio: no lo toco. Añade el reenvío a mano si lo quieres.");
  }
}

writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + "\n");
console.log(`${uninstall ? "Desinstalado" : "Instalado"}. Copia de seguridad: settings.json.backup-${stamp}`);
