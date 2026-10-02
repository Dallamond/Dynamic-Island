# Checklist de pruebas (Lucas)

Lo marcado como "verificado" ya lo comprobó Claude el 02/10/2026 con capturas y medidas.
Lo demás necesita tus manos, tus apps o tus ojos. Marca con `x` lo que funcione y apunta lo que no.

## Instalar

- [ ] Ejecuta `D:\dev-tools\targets\dynamic-island\release\bundle\nsis\Dynamic Island_0.1.0_x64-setup.exe`
      (sin firmar: SmartScreen puede avisar → "Más información" → "Ejecutar de todas formas").
- [ ] Ajustes → General → "Arrancar con Windows" y reinicia: la isla aparece sola.

## Fase 1 · La isla

- [x] Arriba al centro, reloj 24 h, se asoma y se expande con el ratón (verificado).
- [x] Arrastrar e imantar arriba/izquierda/derecha; posición recordada al reiniciar (verificado).
- [ ] Sin parpadeos al expandir/cerrar en tu uso normal (juzga tú).
- [ ] Mover entre los 3 monitores: arrastrando, con `Ctrl+Alt+M` y desde el mapa de Ajustes.
- [ ] En el monitor vertical (1080×1920) se ve y se imanta bien.
- [ ] Atajos: `Ctrl+Alt+I` expandir/cerrar · `Ctrl+Alt+H` ocultar/mostrar. (`Ctrl+Alt+Space` lo usa
      otra app tuya, por eso cambió el de expandir.)
- [ ] La isla no aparece en Alt+Tab ni en la barra de tareas; sí en la bandeja (icono).

## Fase 2 · Música

- [x] Spotify: título, artista, carátula, colores y barras (verificado).
- [ ] Play/pausa, siguiente, anterior y saltar en la barra de progreso.
- [ ] YouTube en Chrome/Edge aparece en la isla.
- [ ] Volumen y silencio desde la isla.
- [ ] Cambiar la salida de audio (FxSound ↔ altavoces/auriculares).

## Fase 3 · Claude Code

- [x] Al trabajar pasa a "Trabajando · acción", con archivos, contexto, 5 h, semana y coste (verificado con esta sesión).
- [ ] Al acabar un turno: "Listo · tiempo" en verde unos segundos.
- [ ] Al pedir permiso: "Necesita tu permiso" en ámbar.
- [ ] Los % de contexto/5 h/semana coinciden con los de `/usage` o la línea de estado.
- [ ] La línea de estado nueva de Claude Code (`Opus · ctx 12% · $0.50 · 5h 21% · 7d 38%`) te gusta.
      Para quitar todo: `node scripts/install-claude-hooks.mjs --uninstall`.

## Fase 4 · Sistema y utilidades

- [x] CPU, RAM y GPU con temperatura, consumo y procesos (verificado).
- [ ] Los números se parecen a los del Administrador de tareas.
- [x] Cuenta atrás sigue con la isla cerrada y avisa al acabar (verificado).
- [ ] Pomodoro completo: suena la campanilla y pasa a descanso.
- [x] Calculadora con teclado: `12×3+4 = 40` (verificado).
- [x] Notas: se guardan en `notes.json` (verificado). Reinicia y comprueba que siguen ahí.

## Fase 5 · Multimonitor y pulido

- [x] Una isla en cada monitor a la vez, con el mismo estado (verificado).
- [x] Pasa a barrita mínima con una ventana a pantalla completa y vuelve al cerrarla (verificado).
- [ ] Prueba con un juego o un vídeo de YouTube a pantalla completa.
- [ ] Opción "Mínima en el monitor que estás usando": ¿te sirve o sobra?
- [x] Consumo medido y apuntado en `ROADMAP.md` (≈ 96 MB y 0 % CPU con una isla).

## Cosas que decidir

- Multimonitor viene **desactivado** (casi duplica la RAM: ~200 MB con 3 islas). Actívalo en Ajustes → Posición.
- El objetivo de < 80 MB no es alcanzable con WebView2; ¿te vale ~96 MB?
