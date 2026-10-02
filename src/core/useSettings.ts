import { useEffect, useState } from "react";
import { ipc, useTauriEvent } from "./ipc";
import type { Settings } from "./types";

/** Ajustes actuales; se actualizan solos cuando Rust emite `settings://changed`. */
export function useSettings(): Settings | null {
  const [settings, setSettings] = useState<Settings | null>(null);
  useEffect(() => {
    ipc.getSettings().then(setSettings);
  }, []);
  useTauriEvent<Settings>("settings://changed", setSettings);
  return settings;
}
