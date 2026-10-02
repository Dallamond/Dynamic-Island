// Registro de módulos. Un módulo = proveedor en Rust + este componente en React.
// - Host: siempre montado si el módulo está activo; escucha a Rust y publica su actividad.
// - Panel: contenido de su pestaña en la isla expandida.
import type { FC } from "react";
import type { LucideIcon } from "lucide-react";
import { House } from "lucide-react";
import type { Settings } from "../core/types";
import { HomePanel } from "./home/HomePanel";

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
];
