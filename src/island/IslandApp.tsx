import { ErrorBoundary } from "../core/ErrorBoundary";
import { useSettings } from "../core/useSettings";
import { MODULES } from "../modules/registry";
import { Island } from "./Island";

/** Monta los Host de los módulos activos (un módulo desactivado no consulta nada) y la isla. */
export function IslandApp() {
  const settings = useSettings();
  if (!settings) return null;
  return (
    <>
      {MODULES.filter((m) => m.Host && m.enabled(settings)).map((m) => {
        const Host = m.Host!;
        return (
          <ErrorBoundary key={m.id}>
            <Host />
          </ErrorBoundary>
        );
      })}
      <Island settings={settings} />
    </>
  );
}
