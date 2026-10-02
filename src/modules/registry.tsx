// Registro de módulos. Un módulo = proveedor en Rust + este componente en React.
// - Host: siempre montado si el módulo está activo; escucha a Rust y publica su actividad.
// - Panel: contenido de su pestaña en la isla expandida.
import type { FC } from "react";
import type { LucideIcon } from "lucide-react";
import { House, Music2, Sparkle } from "lucide-react";
import type { Settings } from "../core/types";
import { HomePanel } from "./home/HomePanel";
import { MediaHost, MediaPanel } from "./media/Media";
import "./media/media.css";
import { AgentHost, AgentPanel } from "./agent/Agent";
import "./agent/agent.css";

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
];
