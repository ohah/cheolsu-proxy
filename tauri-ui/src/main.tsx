import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import { loader } from "@monaco-editor/react";
import App from "./app/App";
import { i18n, defaultLocale, loadCatalog } from "@/shared/lib/i18n";
import { registerContentViewLanguages } from "@/shared/lib/monaco-languages";
import "./main.css";
import "../styles.css";

loader.init().then((monaco) => {
  registerContentViewLanguages(monaco);
});

const container = document.getElementById("root");

if (container) {
  const root = createRoot(container);

  const savedLocale = (localStorage.getItem("locale") as "en" | "ko") || defaultLocale;

  loadCatalog(savedLocale).then(() => {
    root.render(
      <React.StrictMode>
        <I18nProvider i18n={i18n}>
          <App />
        </I18nProvider>
      </React.StrictMode>,
    );
  });
} else {
  console.error("Failed to find the root element");
}
