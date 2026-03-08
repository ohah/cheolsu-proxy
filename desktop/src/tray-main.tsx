import React from "react";
import { createRoot } from "react-dom/client";
import { TrayPanel } from "@/pages/tray-panel";
import "@/pages/tray-panel/ui/tray-panel.css";

const container = document.getElementById("root");

if (container) {
  const root = createRoot(container);
  root.render(
    <React.StrictMode>
      <TrayPanel />
    </React.StrictMode>,
  );
}
