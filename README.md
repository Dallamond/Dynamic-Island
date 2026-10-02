# Dynamic Island para Windows

Isla negra tipo iPhone para Windows: reloj, música (Spotify/YouTube), estado de Claude Code,
CPU/RAM/GPU, pomodoro, cuenta atrás, cronómetro, calculadora y notas. Tauri 2 + Rust + React.

## Uso

- **Instalar:** `Dynamic Island_0.1.0_x64-setup.exe` (se genera en
  `D:\dev-tools\targets\dynamic-island\release\bundle\nsis\`).
- **Ratón:** encima se asoma y se expande; clic la fija abierta; arrastrar la mueve e imanta
  arriba o a un lateral del monitor donde la sueltes.
- **Atajos:** `Ctrl+Alt+I` expandir/cerrar · `Ctrl+Alt+M` siguiente monitor · `Ctrl+Alt+H` ocultar.
  Configurables en Ajustes.
- **Ajustes:** engranaje de la isla expandida o icono de la bandeja.
- **Claude Code:** `node scripts/install-claude-hooks.mjs` (ya instalado el 02/10/2026).

## Documentación

- `CLAUDE.md` — reglas, toolchain y comandos para programar con Claude Code.
- `docs/ROADMAP.md` — fases, estado y mediciones de consumo.
- `docs/PRUEBAS.md` — checklist de pruebas manuales.
- `docs/REFERENCES.md` — repos de referencia y licencias.
