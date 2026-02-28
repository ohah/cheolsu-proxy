import React from "react";
import { createRoot } from "react-dom/client";
import App from "./app/App";
import "./main.css";
import "../styles.css";
import "./shared/stores/session-store";

// react-scan을 개발 환경에서만 실행
if (process.env.NODE_ENV === "development") {
  import("react-scan").then(({ scan }) => {
    scan();
  });
}

const container = document.getElementById("root");

if (container) {
  const root = createRoot(container);
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
} else {
  console.error("Failed to find the root element");
}
