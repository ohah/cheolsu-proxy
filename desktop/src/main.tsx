import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import App from "./app/App";
import { i18n, defaultLocale, loadCatalog } from "@/shared/lib/i18n";
import { migrateLocalStorageToTauriStore } from "@/shared/lib/migrate-localstorage";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import "./main.css";
import "../styles.css";

import("@monaco-editor/react").then(({ loader }) => {
  loader.init().then(async (monaco) => {
    const { registerContentViewLanguages } = await import("@/shared/lib/monaco-languages");
    const { setupMonacoLanguage } = await import("@/features/query-filter-editor/lib/monaco-setup");
    registerContentViewLanguages(monaco);
    setupMonacoLanguage(monaco);
  });
});

/** zustand persist + tauri-store hydration 대기 */
function waitForHydration(): Promise<void> {
  return new Promise((resolve) => {
    const unsub = useAppSettingsStore.persist.onFinishHydration(() => {
      unsub();
      resolve();
    });
    // 이미 hydration이 완료된 경우
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
    // localStorage → tauri-store 마이그레이션 (최초 1회)
    await migrateLocalStorageToTauriStore();

    // store hydration 대기
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
