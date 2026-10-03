# Hoja de ruta

Las fases 1 a 4 forman el MVP. Cada fase termina con su criterio verificado y un commit.
Estado: ⬜ pendiente · 🔄 en curso · ✅ hecha · 🧪 hecha, falta prueba manual.

| Fase | Estado | Se da por terminada cuando |
| --- | --- | --- |
| 0 · Preparación | ✅ | Un repo con Tauri 2 y React abre una ventana con `npm run tauri dev`, y hay un fichero de notas con el código MIT que se reutilizará |
| 1 · La isla | 🧪 | Aparece arriba al centro, se expande con el ratón sin parpadeos, se arrastra y se imanta, cambia de monitor, recuerda su posición al reiniciar y muestra la hora en 24 h |
| 2 · Música | 🧪 | Con Spotify sonando se ven título y carátula, los botones controlan la reproducción, YouTube en el navegador también aparece y se pueden cambiar volumen y salida de audio |
| 3 · Claude Code | 🧪 | Al lanzar Claude Code la isla pasa a "trabajando" y al terminar a "listo"; tokens y uso coinciden con lo que muestra Claude Code; con Claude Code cerrado no consulta nada |
| 4 · Sistema y utilidades | 🧪 | CPU, RAM y GPU coinciden con el Administrador de tareas; pomodoro y cronómetro siguen contando con la isla cerrada; las notas se conservan al reiniciar |
| 5 · Multimonitor y pulido | 🧪 | Hay una isla por monitor a la vez, pasa a estado mínimo con una pantalla completa, el consumo en reposo está medido y apuntado, y hay un instalador |
| 6 · Futuro | 🔄 | Permisos desde la isla, avisos configurables, Codex/agentes locales, letras LRCLIB, Google Calendar, notificaciones de Windows. Cada uno es su propia mini-fase |

### Fase 6 · mini-fases

| Mini-fase | Estado | Notas |
| --- | --- | --- |
| Píldora ampliada (detalle de agentes sin expandir) | ✅ | 03/10/2026 · solo monitor principal, todos o nunca |
| Temporizador junto a los agentes | ✅ | 03/10/2026 · columna propia en la píldora alta |
| Codex | ✅ | 03/10/2026 · lee `~/.codex/sessions`; sin aviso de permiso (Codex no lo registra) |
| IA local (Bionic / LM Studio / Ollama) | ✅ | 03/10/2026 · "generando" por GPU ≥ 60 % (reposo ≤ 18 %, generando 89–100 % en RTX 3060) |
| Varios agentes a la vez | ✅ | 03/10/2026 · una fila por agente (máx. 3) |
| Contenido fijo por monitor | ✅ | 03/10/2026 · automático, agentes, temporizador o música |
| Permisos de Claude Code desde la isla | ⬜ | |
| Avisos configurables | ⬜ | |
| Letras LRCLIB | ⬜ | |
| Google Calendar | ⬜ | |
| Notificaciones de Windows | ⬜ | |

## Funcionalidades por fase

| Función | Fase |
| --- | --- |
| Ventana negra sin borde, transparente, siempre encima, fuera de Alt+Tab | 1 |
| Estados cerrado, hover y expandido con animación fluida | 1 |
| Arrastrar e imantar arriba o en un lateral | 1 |
| Moverla entre los 3 monitores, posición por monitor | 1 |
| Reloj 24 h | 1 |
| Ajustes JSON: aspecto, posición, módulos on/off, atajos | 1 |
| Arranque con Windows y una sola instancia | 1 |
| Canción, carátula, controles y progreso (SMTC) | 2 |
| Volumen y salida de audio (WASAPI/COM) | 2 |
| Estado de Claude Code por hooks HTTP | 3 |
| Tokens y uso por statusLine | 3 |
| Detalle intermedio: última acción, archivos, tiempo de turno | 3 |
| CPU, RAM, GPU (sysinfo + NVML) | 4 |
| Pomodoro, cronómetro, cuenta atrás (en Rust) | 4 |
| Calculadora y notas rápidas | 4 |
| Una isla por monitor sincronizada | 5 |
| Estado mínimo en pantalla completa | 5 |
| Instalador y medición de consumo | 5 |

## Registro de mediciones

02/10/2026 · build release (`npm run tauri build`) · Windows 10, AMD Ryzen 5 3600 6-Core Processor (12 hilos), RTX 3060, 3 monitores.
Suma de `dynamic-island.exe` + sus procesos `msedgewebview2` (memoria privada; el working set cuenta
memoria compartida varias veces). CPU = media sobre todos los núcleos. Spotify sonando y Claude Code activo.

| Escenario | Procesos | RAM privada | CPU media |
| --- | --- | --- | --- |
| 1 isla, cerrada, WebView2 por defecto | 7 | 169 MB | 0,01 % |
| 1 isla, cerrada, **args finales** (`--disable-gpu` y compañía) | 6 | **96 MB** | **0,01 %** |
| 1 isla animando (entrar/salir 5 veces en 9 s), por defecto | 7 | 235 MB | 3,07 % |
| 1 isla animando, args finales | 6 | 135 MB | 1,46 % |
| 3 islas (multimonitor), por defecto | 9 | 280 MB | 0,15 % |
| 3 islas, args finales (una expandida con sistema) | 8 | 197 MB | 0,11 % |

- El `.exe` en sí ocupa 7–28 MB; el resto es WebView2 (proceso navegador, renderer y utilidades).
- El presupuesto inicial (< 80 MB) no se alcanza con WebView2: el suelo realista son ~90 MB con una isla.
  CPU en reposo sí cumple (≈ 0 %).
- Sin GPU las animaciones gastan **menos** CPU (se ahorra el proceso de GPU) y la transparencia funciona.
- Instalador NSIS: 1,5 MB · ejecutable: 4 MB.
