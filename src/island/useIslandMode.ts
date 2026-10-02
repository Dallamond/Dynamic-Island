// Máquina de estados de la isla: cerrada → hover (asomada) → expandida.
// El hover lo detecta Rust (hilo del cursor) y llega como evento `island://hover`.
import { useCallback, useEffect, useRef, useState } from "react";
import { ipc, useTauriEvent } from "../core/ipc";

export type Mode = "collapsed" | "peek" | "expanded";

export function useIslandMode(expandDelayMs: number, collapseDelayMs: number) {
  const [mode, setMode] = useState<Mode>("collapsed");
  const [dragging, setDragging] = useState(false);
  const hovered = useRef(false);
  const pinned = useRef(false); // expandida por atajo o clic: no se cierra hasta salir
  const editing = useRef(false); // escribiendo en notas/calculadora: no cerrar al salir el ratón
  const focusable = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const justDragged = useRef(false);

  const clear = () => window.clearTimeout(timer.current);
  const scheduleCollapse = (delay: number) => {
    clear();
    timer.current = window.setTimeout(() => {
      pinned.current = false;
      setMode("collapsed");
    }, delay);
  };

  useTauriEvent<boolean>("island://hover", (inside) => {
    hovered.current = inside;
    clear();
    if (inside) {
      setMode((m) => (m === "expanded" ? m : "peek"));
      timer.current = window.setTimeout(() => setMode("expanded"), expandDelayMs);
    } else if (!editing.current) {
      scheduleCollapse(collapseDelayMs);
    }
  });

  useTauriEvent<boolean>("island://drag", (d) => {
    setDragging(d);
    if (d) {
      clear();
      justDragged.current = true;
      setMode("collapsed");
    } else {
      // El clic que llega justo al soltar no debe expandir.
      window.setTimeout(() => (justDragged.current = false), 50);
    }
  });

  useTauriEvent<null>("island://toggle", () => {
    clear();
    setMode((m) => {
      if (m === "expanded") {
        pinned.current = false;
        return "collapsed";
      }
      pinned.current = true;
      return "expanded";
    });
  });

  /** Pide el foco de teclado a Windows (la isla es NOACTIVATE por defecto). */
  const startEditing = useCallback(() => {
    editing.current = true;
    if (!focusable.current) {
      focusable.current = true;
      ipc.setFocusable(true);
    }
  }, []);

  /** El foco salió del campo de texto: deja de editar y, sin ratón encima, cierra. */
  const stopEditing = useCallback(() => {
    if (!editing.current) return;
    editing.current = false;
    if (!hovered.current) scheduleCollapse(collapseDelayMs);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [collapseDelayMs]);

  // Al perder el foco (clic fuera de la isla) se deja de editar y, si no hay ratón encima, se cierra.
  useEffect(() => {
    const onBlur = () => {
      if (!editing.current) return;
      editing.current = false;
      if (!hovered.current) scheduleCollapse(150);
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cerrada: devolver el estilo NOACTIVATE para no robar el foco en los clics normales.
  useEffect(() => {
    if (mode === "collapsed" && focusable.current) {
      focusable.current = false;
      editing.current = false;
      ipc.setFocusable(false);
    }
  }, [mode]);

  const onClick = useCallback(() => {
    if (justDragged.current) return;
    clear();
    pinned.current = true;
    setMode("expanded");
  }, []);

  useEffect(() => clear, []);

  return { mode, dragging, onClick, startEditing, stopEditing };
}
