import React from "react";
import ReactDOM from "react-dom/client";
import App from "@/app/App";
import OverlayApp from "@/features/overlay/OverlayApp";
import { AppProviders } from "@/app/providers";
import { installProductionDevToolsGuards } from "@/shared/lib/preventProductionDevTools";
import { revealAppWindow } from "@/shared/lib/revealAppWindow";
import { applyThemePreference, readStoredThemePreference } from "@/shared/lib/theme";
import "./index.css";

installProductionDevToolsGuards();
applyThemePreference(readStoredThemePreference());

const isOverlay = window.location.hash.startsWith("#/overlay");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppProviders overlay={isOverlay}>
      {isOverlay ? <OverlayApp /> : <App />}
    </AppProviders>
  </React.StrictMode>,
);

if (!isOverlay) revealAppWindow();
