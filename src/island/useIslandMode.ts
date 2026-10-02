// Máquina de estados de la isla: cerrada → hover (asomada) → expandida.
// El hover lo detecta Rust (hilo del cursor) y llega como evento `island://hover`.
import { useCallback, useEffect, useRef, useState } from "react";
import { useTauriEvent } from "../core/ipc";

export type Mode = "collapsed" | "peek" | "expanded";

export function useIslandMode(expandDelayMs: number, collapseDelayMs: number) {
  const [mode, setMode] = useState<Mode>("collapsed");
  const [dragging, setDragging] = useState(false);
  const hovered = useRef(false);
  const pinned = useRef(false); // expandida por atajo o clic: no se cierra hasta salir
  const timer = useRef<number | undefined>(undefined);
  const justDragged = useRef(false);

  const clear = () => window.clearTimeout(timer.current);

  useTauriEvent<boolean>("island://hover", (inside) => {
    hovered.current = inside;
    clear();
    if (inside) {
      setMode((m) => (m === "expanded" ? m : "peek"));
      timer.current = window.setTimeout(() => setMode("expanded"), expandDelayMs);
    } else {
      timer.current = window.setTimeout(() => {
        pinned.current = false;
        setMode("collapsed");
      }, collapseDelayMs);
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

  const onClick = useCallback(() => {
    if (justDragged.current) return;
    clear();
    pinned.current = true;
    setMode("expanded");
  }, []);

  useEffect(() => clear, []);

  return { mode, dragging, onClick };
}
