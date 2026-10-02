import { invoke } from "@tauri-apps/api/core";
import { createStore } from "../../core/store";

export interface MediaState {
  appId: string;
  appName: string;
  title: string;
  artist: string;
  album: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  updatedAtMs: number;
  canPrev: boolean;
  canNext: boolean;
  canSeek: boolean;
  thumbnail: string | null;
}

export interface MediaView {
  state: MediaState | null;
  /** Momento local en que llegó el estado (respaldo si la app no da `updatedAtMs`). */
  receivedAt: number;
  /** Color dominante de la carátula, para acentos. */
  color: string | null;
}

export const mediaStore = createStore<MediaView>({ state: null, receivedAt: 0, color: null });

export const media = {
  control: (action: "playPause" | "next" | "prev" | "seek", positionMs?: number) =>
    invoke("media_control", { action, positionMs }),
  refresh: () => invoke("media_refresh"),
};

/** Posición actual estimada: la última conocida más el tiempo transcurrido si está sonando. */
export function currentPosition(v: MediaView, now = Date.now()): number {
  const s = v.state;
  if (!s) return 0;
  if (!s.playing) return s.positionMs;
  const base = s.updatedAtMs > 0 && s.updatedAtMs <= now ? s.updatedAtMs : v.receivedAt;
  const pos = s.positionMs + (now - base);
  return s.durationMs > 0 ? Math.min(pos, s.durationMs) : pos;
}

export function fmtMs(ms: number) {
  const t = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(t / 60);
  const s = t % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/** Color medio de la carátula, con algo más de saturación para que luzca sobre negro. */
export function dominantColor(src: string): Promise<string | null> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = c.height = 12;
      const ctx = c.getContext("2d");
      if (!ctx) return resolve(null);
      ctx.drawImage(img, 0, 0, 12, 12);
      const d = ctx.getImageData(0, 0, 12, 12).data;
      let r = 0, g = 0, b = 0, n = 0;
      for (let i = 0; i < d.length; i += 4) {
        const max = Math.max(d[i], d[i + 1], d[i + 2]);
        const min = Math.min(d[i], d[i + 1], d[i + 2]);
        // Ignora píxeles casi grises o muy oscuros: no dan color.
        if (max < 40 || max - min < 25) continue;
        r += d[i]; g += d[i + 1]; b += d[i + 2]; n++;
      }
      if (n === 0) return resolve(null);
      r /= n; g /= n; b /= n;
      const boost = (v: number) => Math.round(Math.min(255, v * 1.25 + 20));
      resolve(`rgb(${boost(r)}, ${boost(g)}, ${boost(b)})`);
    };
    img.onerror = () => resolve(null);
    img.src = src;
  });
}
