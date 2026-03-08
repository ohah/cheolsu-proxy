import { useEffect } from "react";
import { ThemeProvider, RouterProvider } from "./providers";
import { Toaster } from "@/shared/ui";
import {
  useProxyStore,
  useInterceptRuleStore,
  useTransactionStore,
  useWebSocketStore,
  useMapRuleStore,
  useScriptStore,
} from "@/shared/stores";
import { trayStore } from "@/shared/stores/tray-sync-store";
import { listen } from "@tauri-apps/api/event";
import type { ProxyEventTuple } from "@/entities/proxy";
import type { WsMessageInfo, WsConnectionEvent } from "@/entities/websocket";
import type { InterceptRule } from "@/entities/intercept-rule";

const App: React.FC = () => {
  const initializeProxy = useProxyStore((s) => s.initializeProxy);
  const syncToProxy = useInterceptRuleStore((s) => s.syncToProxy);
  const addTransaction = useTransactionStore((s) => s.addTransaction);
  const paused = useTransactionStore((s) => s.paused);
  const addWsMessage = useWebSocketStore((s) => s.addMessage);
  const updateWsConnection = useWebSocketStore((s) => s.updateConnection);
  const setInterceptRules = useInterceptRuleStore((s) => s.setRules);
  const setMapRules = useMapRuleStore((s) => s.setRules);
  const setScriptStatus = useScriptStore((s) => s.setStatus);
  const addScriptLog = useScriptStore((s) => s.addLog);

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

  // 스크립트 이벤트 수신
  useEffect(() => {
    const unlistenLog = listen<{ level: string; message: string }>("script_log", (event) => {
      addScriptLog(event.payload.level, event.payload.message);
    });
    const unlistenStatus = listen<{ active: boolean; path: string | null }>(
      "script_status",
      (event) => {
        setScriptStatus(event.payload.active, event.payload.path);
      },
    );

    return () => {
      unlistenLog.then((f) => f());
      unlistenStatus.then((f) => f());
    };
  }, [addScriptLog, setScriptStatus]);

  // 트레이 패널에서 보내는 이벤트 수신
  const clearTransactions = useTransactionStore((s) => s.clearTransactions);
  const togglePause = useTransactionStore((s) => s.togglePause);
  const transactions = useTransactionStore((s) => s.transactions);

  useEffect(() => {
    const unlistenPause = listen("tray_toggle_pause", () => {
      togglePause();
    });
    const unlistenClear = listen("tray_clear_session", () => {
      clearTransactions();
    });

    return () => {
      unlistenPause.then((f) => f());
      unlistenClear.then((f) => f());
    };
  }, [clearTransactions, togglePause]);

  // 메인 윈도우 상태를 Tauri Store에 동기화 (트레이 패널이 읽음)
  useEffect(() => {
    trayStore.set("paused", paused);
    trayStore.set("transactionCount", transactions.length);
    trayStore.save();
  }, [paused, transactions.length]);

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
