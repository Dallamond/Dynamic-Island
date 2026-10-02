// Punto de entrada único: la ventana "main" es la isla, "settings" la ventana de ajustes.
import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

async function boot() {
  if (getCurrentWindow().label === "settings") {
    await import("./settings/settings.css");
    const { SettingsApp } = await import("./settings/SettingsApp");
    root.render(
      <React.StrictMode>
        <SettingsApp />
      </React.StrictMode>,
    );
  } else {
    await import("./island/island.css");
    const { IslandApp } = await import("./island/IslandApp");
    root.render(
      <React.StrictMode>
        <IslandApp />
      </React.StrictMode>,
    );
  }
}

boot();
