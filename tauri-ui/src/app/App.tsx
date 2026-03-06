import { useEffect } from "react";
import { useThemeProvider, RouterProvider } from "./providers";
import { Toaster } from "@/shared/ui";
import { useProxyStore, useInterceptRuleStore, useTransactionStore } from "@/shared/stores";
import { listen } from "@tauri-apps/api/event";
import type { ProxyEventTuple } from "@/entities/proxy";

const App: React.FC = () => {
  useThemeProvider();
  const initializeProxy = useProxyStore((s) => s.initializeProxy);
  const syncToProxy = useInterceptRuleStore((s) => s.syncToProxy);
  const addTransaction = useTransactionStore((s) => s.addTransaction);
  const paused = useTransactionStore((s) => s.paused);

  // 앱 시작 시 프록시 초기화 후 저장된 인터셉트 규칙 동기화
  useEffect(() => {
    initializeProxy().then(() => syncToProxy());
  }, [initializeProxy, syncToProxy]);

  // 프록시 이벤트를 전역적으로 수신하여 트랜잭션 store에 저장
  useEffect(() => {
    if (paused) return;

    const unlisten = listen<ProxyEventTuple>("proxy_event", (event) => {
      const [request, response] = event.payload;
      addTransaction({ request, response });
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [addTransaction, paused]);

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

export default App;
