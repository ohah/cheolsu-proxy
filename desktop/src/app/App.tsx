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

const THEME_VALUE_MAP = Object.fromEntries([
  ["light", "light"],
  ["dark", "dark"],
  ...CUSTOM_THEME_KEYS.map((key) => [key, `dark ${key}`]),
]);

const App: React.FC = () => {
  useGlobalShortcut();
  useProxyEventListeners();
  useWebSocketListeners();
  useSseListeners();
  useDaemonSyncListeners();
  useScriptListeners();
  useSessionPersistence();
  useMenuNavigation();

  return (
    <ThemeProvider
      attribute={["class", "data-theme"]}
      defaultTheme="system"
      enableSystem
      themes={["light", "dark", "system", ...CUSTOM_THEME_KEYS]}
      value={THEME_VALUE_MAP}
    >
      <div className="App">
        <RouterProvider />
        <Toaster richColors />
      </div>
    </ThemeProvider>
  );
};

export default App;
