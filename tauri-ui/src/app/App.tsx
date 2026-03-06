import { useEffect } from "react";
import { useThemeProvider, RouterProvider } from "./providers";
import { Toaster } from "@/shared/ui";
import { useProxyStore, useInterceptRuleStore } from "@/shared/stores";

const App: React.FC = () => {
  useThemeProvider();
  const { initializeProxy } = useProxyStore();
  const syncToProxy = useInterceptRuleStore((s) => s.syncToProxy);

  // 앱 시작 시 프록시 초기화 후 저장된 인터셉트 규칙 동기화
  useEffect(() => {
    initializeProxy().then(() => syncToProxy());
  }, [initializeProxy, syncToProxy]);

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

export default App;
