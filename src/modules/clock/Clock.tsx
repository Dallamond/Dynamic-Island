// Reloj 24 h. Un único temporizador alineado al segundo/minuto: coste prácticamente nulo.
import { useEffect, useState } from "react";

export function useNow(withSeconds: boolean): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    let t: number;
    const tick = () => {
      const d = new Date();
      setNow(d);
      const ms = withSeconds ? 1000 - d.getMilliseconds() : 60_000 - d.getSeconds() * 1000 - d.getMilliseconds();
      t = window.setTimeout(tick, ms + 5);
    };
    tick();
    return () => window.clearTimeout(t);
  }, [withSeconds]);
  return now;
}

const pad = (n: number) => n.toString().padStart(2, "0");

export function formatTime(d: Date, withSeconds: boolean) {
  const hm = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  return withSeconds ? `${hm}:${pad(d.getSeconds())}` : hm;
}

const dateFmt = new Intl.DateTimeFormat("es-ES", { weekday: "long", day: "numeric", month: "long" });

export function CompactClock({ seconds }: { seconds: boolean }) {
  const now = useNow(seconds);
  return <span className="clock-compact">{formatTime(now, seconds)}</span>;
}

/** Reloj grande para la vista de inicio expandida. */
export function BigClock() {
  const now = useNow(true);
  return (
    <div className="clock-big">
      <div className="clock-big-time">
        {formatTime(now, false)}
        <span className="clock-big-sec">{pad(now.getSeconds())}</span>
      </div>
      <div className="clock-big-date">{capitalize(dateFmt.format(now))}</div>
    </div>
  );
}

const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
