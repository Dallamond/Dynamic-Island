// "Actividades en vivo": lo que un módulo quiere enseñar en la píldora cerrada.
// La de mayor prioridad ocupa la píldora; la segunda aparece como indicador pequeño.
import { useEffect, type FC } from "react";
import { createStore, useStore } from "./store";

export interface Activity {
  id: string;
  /** Mayor = más importante. Agente esperando permiso > agente trabajando > temporizador > música. */
  priority: number;
  /** Contenido de la píldora cerrada. */
  Compact: FC;
  /** Indicador mínimo cuando es la actividad secundaria (un icono o un número). */
  Badge?: FC;
  /** Módulo al que saltar al expandir con esta actividad delante. */
  tab?: string;
}

const activities = createStore<Activity[]>([]);

/** Registra (o retira, con `null`) la actividad de un módulo. */
export function useActivity(activity: Activity | null) {
  const id = activity?.id;
  const priority = activity?.priority;
  const Compact = activity?.Compact;
  const Badge = activity?.Badge;
  const tab = activity?.tab;
  useEffect(() => {
    if (!id || !Compact || priority === undefined) return;
    const a: Activity = { id, priority, Compact, Badge, tab };
    activities.set((list) => [...list.filter((x) => x.id !== id), a].sort((x, y) => y.priority - x.priority));
    return () => activities.set((list) => list.filter((x) => x.id !== id));
  }, [id, priority, Compact, Badge, tab]);
}

export function useActivities(): Activity[] {
  return useStore(activities);
}
