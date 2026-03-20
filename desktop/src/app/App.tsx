import { ThemeProvider, RouterProvider } from "./providers";
import { Toaster } from "@/shared/ui";
import { useGlobalShortcut } from "@/features/proxy-toggle";
import {
  useProxyEventListeners,
  useWebSocketListeners,
  useSseListeners,
  useDaemonSyncListeners,
  useScriptListeners,
  useSessionPersistence,
  useMenuNavigation,
} from "./hooks";
import { CUSTOM_THEME_KEYS } from "@/features/query-filter-editor/model/themes";
import { useTheme } from "next-themes";
import { useEffect } from "react";

const CUSTOM_THEME_SET = new Set<string>(CUSTOM_THEME_KEYS);

/** Sync dark + theme-specific classes on <html> based on resolved theme */
function useThemeClassSync() {
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    const el = document.documentElement;
    el.classList.toggle(
      "dark",
      resolvedTheme === "dark" || CUSTOM_THEME_SET.has(resolvedTheme ?? ""),
    );
    for (const key of CUSTOM_THEME_KEYS) {
      el.classList.toggle(key, resolvedTheme === key);
    }
  }, [resolvedTheme]);
}

const AppContent: React.FC = () => {
  useThemeClassSync();
  useGlobalShortcut();
  useProxyEventListeners();
  useWebSocketListeners();
  useSseListeners();
  useDaemonSyncListeners();
  useScriptListeners();
  useSessionPersistence();
  useMenuNavigation();

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

const App: React.FC = () => (
  <ThemeProvider
    attribute="data-theme"
    defaultTheme="system"
    enableSystem
    themes={["light", "dark", "system", ...CUSTOM_THEME_KEYS]}
  >
    <AppContent />
  </ThemeProvider>
);

export default App;
