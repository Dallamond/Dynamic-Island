// Módulo de música: actividad en la píldora mientras suena algo y panel con controles.
import { ChevronDown, Music2, Pause, Play, SkipBack, SkipForward, Speaker, Volume1, Volume2, VolumeX } from "lucide-react";
import { useEffect, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useActivity } from "../../core/activities";
import { useTauriEvent } from "../../core/ipc";
import { useStore } from "../../core/store";
import { currentPosition, dominantColor, fmtMs, media, mediaStore, type MediaState } from "./store";

// ---------------------------------------------------------------- Host

export function MediaHost() {
  useTauriEvent<MediaState | null>("media://state", (state) => {
    const prevThumb = mediaStore.get().state?.thumbnail;
    mediaStore.set((v) => ({ ...v, state, receivedAt: Date.now() }));
    if (state?.thumbnail && state.thumbnail !== prevThumb) {
      dominantColor(state.thumbnail).then((color) => mediaStore.set((v) => ({ ...v, color })));
    } else if (!state?.thumbnail) {
      mediaStore.set((v) => (v.color ? { ...v, color: null } : v));
    }
  });
  useEffect(() => {
    media.refresh();
  }, []);

  const playing = useStore(mediaStore).state?.playing ?? false;
  useActivity(playing ? { id: "media", priority: 10, Compact: MediaCompact, Badge: MediaBadge, tab: "media" } : null);
  return null;
}

// ---------------------------------------------------------------- Píldora

function Art({ size, radius }: { size: number; radius: number }) {
  const { state } = useStore(mediaStore);
  const style: CSSProperties = { width: size, height: size, borderRadius: radius };
  return state?.thumbnail ? (
    <img className="media-art" src={state.thumbnail} style={style} draggable={false} />
  ) : (
    <span className="media-art placeholder" style={style}>
      <Music2 size={size * 0.5} />
    </span>
  );
}

function Bars({ playing }: { playing: boolean }) {
  const { color } = useStore(mediaStore);
  return (
    <span className={`media-bars${playing ? " on" : ""}`} style={{ "--bar": color ?? "var(--accent)" } as CSSProperties}>
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}

function MediaCompact() {
  const { state } = useStore(mediaStore);
  if (!state) return null;
  return (
    <span className="media-compact">
      <Art size={22} radius={6} />
      <span className="media-compact-text">
        {state.title}
        {state.artist && <span className="muted"> · {state.artist}</span>}
      </span>
      <Bars playing={state.playing} />
    </span>
  );
}

function MediaBadge() {
  return <Bars playing />;
}

// ---------------------------------------------------------------- Panel

export function MediaPanel() {
  const view = useStore(mediaStore);
  const { state, color } = view;
  const [, force] = useState(0);

  // La barra avanza sola mientras el panel está abierto; cerrado no hay temporizador.
  useEffect(() => {
    if (!state?.playing) return;
    const t = window.setInterval(() => force((n) => n + 1), 500);
    return () => window.clearInterval(t);
  }, [state?.playing]);

  if (!state) {
    return (
      <div className="media-empty">
        <Music2 size={28} className="muted" />
        <p className="muted">No suena nada. Abre Spotify o un vídeo en el navegador.</p>
        <AudioControls />
      </div>
    );
  }

  const pos = currentPosition(view);
  const pct = state.durationMs > 0 ? (pos / state.durationMs) * 100 : 0;
  const seek = (e: MouseEvent<HTMLDivElement>) => {
    if (!state.canSeek || state.durationMs <= 0) return;
    const r = e.currentTarget.getBoundingClientRect();
    media.control("seek", Math.round(((e.clientX - r.left) / r.width) * state.durationMs));
  };

  return (
    <div className="media-panel" style={{ "--tint": color ?? "var(--accent)" } as CSSProperties}>
      <div className="media-top">
        <Art size={84} radius={14} />
        <div className="media-info">
          <span className="label">{state.appName}</span>
          <div className="media-title">{state.title || "Sin título"}</div>
          <div className="media-artist muted">{state.artist}</div>
        </div>
        <Bars playing={state.playing} />
      </div>

      <div className="media-progress">
        <span>{fmtMs(pos)}</span>
        <div className={`media-track${state.canSeek ? " seekable" : ""}`} onClick={seek} data-nodrag>
          <div className="media-fill" style={{ width: `${pct}%` }} />
        </div>
        <span>{state.durationMs > 0 ? fmtMs(state.durationMs) : "--:--"}</span>
      </div>

      <div className="media-controls">
        <button className="btn" style={{ width: 38, height: 38 }} disabled={!state.canPrev} onClick={() => media.control("prev")}>
          <SkipBack size={20} fill="currentColor" />
        </button>
        <button className="btn" style={{ width: 46, height: 46 }} onClick={() => media.control("playPause")}>
          {state.playing ? <Pause size={26} fill="currentColor" /> : <Play size={26} fill="currentColor" />}
        </button>
        <button className="btn" style={{ width: 38, height: 38 }} disabled={!state.canNext} onClick={() => media.control("next")}>
          <SkipForward size={20} fill="currentColor" />
        </button>
      </div>

      <AudioControls />
    </div>
  );
}

// ---------------------------------------------------------------- Volumen y salida

interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}
interface AudioState {
  volume: number;
  muted: boolean;
  devices: AudioDevice[];
}

function AudioControls() {
  const [audio, setAudio] = useState<AudioState | null>(null);
  const [open, setOpen] = useState(false);
  const dragging = useRef(false);
  const pending = useRef<number | undefined>(undefined);

  // Solo se consulta con el panel abierto; cada 1,5 s para reflejar cambios externos.
  useEffect(() => {
    const load = () => {
      if (!dragging.current) invoke<AudioState>("audio_state").then(setAudio).catch(() => {});
    };
    load();
    const t = window.setInterval(load, 1500);
    return () => window.clearInterval(t);
  }, []);

  if (!audio) return null;
  const current = audio.devices.find((d) => d.isDefault);
  const VolIcon = audio.muted || audio.volume === 0 ? VolumeX : audio.volume < 0.5 ? Volume1 : Volume2;

  const setVolume = (v: number) => {
    setAudio({ ...audio, volume: v, muted: false });
    window.clearTimeout(pending.current);
    pending.current = window.setTimeout(() => {
      invoke("audio_set_volume", { volume: v });
      if (audio.muted) invoke("audio_set_mute", { muted: false });
    }, 30);
  };

  return (
    <div className="audio">
      <button className="btn" style={{ width: 28, height: 28 }} title="Silenciar" onClick={() => {
        setAudio({ ...audio, muted: !audio.muted });
        invoke("audio_set_mute", { muted: !audio.muted });
      }}>
        <VolIcon size={16} />
      </button>
      <input
        className="audio-slider"
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={audio.muted ? 0 : audio.volume}
        style={{ "--pct": `${(audio.muted ? 0 : audio.volume) * 100}%` } as CSSProperties}
        onPointerDown={() => (dragging.current = true)}
        onPointerUp={() => (dragging.current = false)}
        onChange={(e) => setVolume(Number(e.target.value))}
      />
      <span className="audio-pct">{Math.round((audio.muted ? 0 : audio.volume) * 100)}</span>
      <div className="audio-device">
        <button className="audio-device-btn" onClick={() => setOpen(!open)} title="Salida de audio">
          <Speaker size={14} />
          <span>{current?.name ?? "Salida"}</span>
          <ChevronDown size={13} />
        </button>
        {open && (
          <div className="audio-menu">
            {audio.devices.map((d) => (
              <button
                key={d.id}
                className={d.isDefault ? "active" : ""}
                onClick={() => {
                  setOpen(false);
                  invoke("audio_set_device", { id: d.id }).then(() => invoke<AudioState>("audio_state").then(setAudio));
                }}
              >
                {d.name}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
