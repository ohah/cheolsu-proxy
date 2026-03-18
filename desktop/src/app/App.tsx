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
    <ThemeProvider attribute={["class", "data-theme"]} defaultTheme="system" enableSystem>
      <div className="App">
        <RouterProvider />
        <Toaster richColors />
      </div>
    </ThemeProvider>
  );
};

export default App;
