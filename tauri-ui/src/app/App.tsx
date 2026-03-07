import { useEffect } from "react";
import { useThemeProvider, RouterProvider } from "./providers";
import { Toaster } from "@/shared/ui";
import {
  useProxyStore,
  useInterceptRuleStore,
  useTransactionStore,
  useWebSocketStore,
  useMapRuleStore,
} from "@/shared/stores";
import { listen } from "@tauri-apps/api/event";
import type { ProxyEventTuple } from "@/entities/proxy";
import type { WsMessageInfo, WsConnectionEvent } from "@/entities/websocket";
import type { InterceptRule } from "@/entities/intercept-rule";

const App: React.FC = () => {
  useThemeProvider();
  const initializeProxy = useProxyStore((s) => s.initializeProxy);
  const syncToProxy = useInterceptRuleStore((s) => s.syncToProxy);
  const addTransaction = useTransactionStore((s) => s.addTransaction);
  const paused = useTransactionStore((s) => s.paused);
  const addWsMessage = useWebSocketStore((s) => s.addMessage);
  const updateWsConnection = useWebSocketStore((s) => s.updateConnection);
  const setInterceptRules = useInterceptRuleStore((s) => s.setRules);
  const setMapRules = useMapRuleStore((s) => s.setRules);

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

  // WebSocket 메시지 이벤트 수신
  useEffect(() => {
    if (paused) return;

    const unlistenMsg = listen<WsMessageInfo>("ws_message", (event) => {
      addWsMessage(event.payload);
    });
    const unlistenConn = listen<WsConnectionEvent>("ws_connection", (event) => {
      const { connection_id, status, uri, time } = event.payload;
      updateWsConnection(connection_id, status, uri, time);
    });

    return () => {
      unlistenMsg.then((f) => f());
      unlistenConn.then((f) => f());
    };
  }, [addWsMessage, updateWsConnection, paused]);

  // 데몬에서 인터셉트 규칙 변경 수신 (MCP 등 외부 클라이언트에서 변경 시 동기화)
  useEffect(() => {
    const unlisten = listen<InterceptRule[]>("intercept_rules_updated", (event) => {
      const rules = event.payload;
      const interceptRules = rules.filter(
        (r) =>
          r.action.type === "block" ||
          r.action.type === "modify_request" ||
          r.action.type === "modify_response",
      );
      const mapRules = rules.filter(
        (r) => r.action.type === "map_local" || r.action.type === "map_remote",
      );
      setInterceptRules(interceptRules);
      setMapRules(mapRules);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setInterceptRules, setMapRules]);

  return (
    <div className="App">
      <RouterProvider />
      <Toaster richColors />
    </div>
  );
};

export default App;
