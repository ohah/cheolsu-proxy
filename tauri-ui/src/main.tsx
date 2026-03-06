import React from "react";
import { createRoot } from "react-dom/client";
import { loader } from "@monaco-editor/react";
import App from "./app/App";
import { registerContentViewLanguages } from "@/shared/lib/monaco-languages";
import "./main.css";
import "../styles.css";

loader.init().then((monaco) => {
  registerContentViewLanguages(monaco);
});

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
