// Registro de módulos. Un módulo = proveedor en Rust + este componente en React.
// - Host: siempre montado si el módulo está activo; escucha a Rust y publica su actividad.
// - Panel: contenido de su pestaña en la isla expandida.
import type { FC } from "react";
import type { LucideIcon } from "lucide-react";
import { Activity, Calculator, House, Music2, NotebookPen, Sparkle, Timer } from "lucide-react";
import type { Settings } from "../core/types";
import { HomePanel } from "./home/HomePanel";
import { MediaHost, MediaPanel } from "./media/Media";
import { AgentHost, AgentPanel } from "./agent/Agent";
import { TimerHost, TimerPanel } from "./timer/Timer";
import { SystemPanel } from "./system/System";
import { CalcPanel } from "./calc/Calc";
import { NotesPanel } from "./notes/Notes";
import "./media/media.css";
import "./agent/agent.css";
import "./utils.css";

export interface IslandModule {
  id: string;
  label: string;
  icon: LucideIcon;
  enabled: (s: Settings) => boolean;
  Host?: FC;
  Panel: FC;
}

export const MODULES: IslandModule[] = [
  { id: "home", label: "Inicio", icon: House, enabled: () => true, Panel: HomePanel },
  { id: "media", label: "Música", icon: Music2, enabled: (s) => s.modules.media, Host: MediaHost, Panel: MediaPanel },
  { id: "agent", label: "Claude Code", icon: Sparkle, enabled: (s) => s.modules.agent, Host: AgentHost, Panel: AgentPanel },
  { id: "timer", label: "Temporizadores", icon: Timer, enabled: (s) => s.modules.timer, Host: TimerHost, Panel: TimerPanel },
  { id: "system", label: "Sistema", icon: Activity, enabled: (s) => s.modules.system, Panel: SystemPanel },
  { id: "calc", label: "Calculadora", icon: Calculator, enabled: (s) => s.modules.calc, Panel: CalcPanel },
  { id: "notes", label: "Notas", icon: NotebookPen, enabled: (s) => s.modules.notes, Panel: NotesPanel },
];
