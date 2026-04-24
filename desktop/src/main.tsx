import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import App from "./app/App";
import { i18n, defaultLocale, loadCatalog } from "@/shared/lib/i18n";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { registerContentViewLanguages } from "@/shared/lib/monaco-languages";
import { setupMonacoLanguage } from "@/features/query-filter-editor/lib/monaco-setup";
import { setupMonacoThemes } from "@/shared/lib/monaco-theme";
import "./main.css";
import "../styles.css";

import("@monaco-editor/react").then(({ loader }) => {
  loader.init().then((monaco) => {
    registerContentViewLanguages(monaco);
    setupMonacoLanguage(monaco);
    setupMonacoThemes(monaco);
  });
});

/** zustand persist + tauri-store hydration 대기 */
function waitForHydration(): Promise<void> {
  return new Promise((resolve) => {
    const unsub = useAppSettingsStore.persist.onFinishHydration(() => {
      unsub();
      resolve();
    });
    if (useAppSettingsStore.persist.hasHydrated()) {
      unsub();
      resolve();
    }
  });
}

const container = document.getElementById("root");

if (container) {
  const root = createRoot(container);

  (async () => {
    await waitForHydration();

    const savedLocale = useAppSettingsStore.getState().locale || defaultLocale;

    await loadCatalog(savedLocale);
    root.render(
      <React.StrictMode>
        <I18nProvider i18n={i18n}>
          <App />
        </I18nProvider>
      </React.StrictMode>,
    );
  })();
} else {
  console.error("Failed to find the root element");
}
