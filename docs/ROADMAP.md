# Hoja de ruta

Las fases 1 a 4 forman el MVP. Cada fase termina con su criterio verificado y un commit.
Estado: ⬜ pendiente · 🔄 en curso · ✅ hecha · 🧪 hecha, falta prueba manual de Lucas.

| Fase | Estado | Se da por terminada cuando |
| --- | --- | --- |
| 0 · Preparación | ✅ | Un repo con Tauri 2 y React abre una ventana con `npm run tauri dev`, y hay un fichero de notas con el código MIT que se reutilizará |
| 1 · La isla | 🧪 | Aparece arriba al centro, se expande con el ratón sin parpadeos, se arrastra y se imanta, cambia de monitor, recuerda su posición al reiniciar y muestra la hora en 24 h |
| 2 · Música | 🧪 | Con Spotify sonando se ven título y carátula, los botones controlan la reproducción, YouTube en el navegador también aparece y se pueden cambiar volumen y salida de audio |
| 3 · Claude Code | ⬜ | Al lanzar Claude Code la isla pasa a "trabajando" y al terminar a "listo"; tokens y uso coinciden con lo que muestra Claude Code; con Claude Code cerrado no consulta nada |
| 4 · Sistema y utilidades | ⬜ | CPU, RAM y GPU coinciden con el Administrador de tareas; pomodoro y cronómetro siguen contando con la isla cerrada; las notas se conservan al reiniciar |
| 5 · Multimonitor y pulido | ⬜ | Hay una isla por monitor a la vez, pasa a estado mínimo con una pantalla completa, el consumo en reposo está medido y apuntado, y hay un instalador |
| 6 · Futuro | ⬜ | Permisos desde la isla, avisos configurables, Codex/agentes locales, letras LRCLIB, Google Calendar, notificaciones de Windows. Cada uno es su propia mini-fase |

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

(Se rellena en la fase 5.)
