// Envoltorio tipado de los comandos y eventos del núcleo en Rust.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type { Layout, MonitorInfo, Settings } from "./types";

export const ipc = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  getLayout: () => invoke<Layout>("get_layout"),
  setHitRect: (x: number, y: number, w: number, h: number) => invoke<void>("set_hit_rect", { x, y, w, h }),
  dragBegin: () => invoke<void>("drag_begin"),
  setFocusable: (focusable: boolean) => invoke<void>("set_focusable", { focusable }),
  openSettings: () => invoke<void>("open_settings"),
  listMonitors: () => invoke<MonitorInfo[]>("list_monitors"),
  moveToMonitor: (name: string) => invoke<void>("move_to_monitor", { name }),
  resetPosition: () => invoke<void>("reset_position"),
  quit: () => invoke<void>("quit"),
};

/** Suscripción a un evento de Rust que se limpia sola al desmontar. */
export function useTauriEvent<T>(name: string, handler: (payload: T) => void) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    let un: UnlistenFn | undefined;
    let cancelled = false;
    listen<T>(name, (e) => ref.current(e.payload)).then((u) => {
      if (cancelled) u();
      else un = u;
    });
    return () => {
      cancelled = true;
      un?.();
    };
  }, [name]);
}
