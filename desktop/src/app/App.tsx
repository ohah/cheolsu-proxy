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
  useBreakpointStore,
  useHostMappingStore,
} from "@/shared/stores";
import { trayStore } from "@/shared/stores/tray-sync-store";
import { listen } from "@tauri-apps/api/event";
import type { ProxyEventTuple } from "@/entities/proxy";
import type { WsMessageInfo, WsConnectionEvent } from "@/entities/websocket";
import type { InterceptRule } from "@/entities/intercept-rule";
import type { BreakpointRule, PendingBreakpoint } from "@/entities/breakpoint";
import type { HostMapping } from "@/shared/api/proxy";
import { useGlobalShortcut } from "@/features/proxy-toggle";
import { updateDaemonRules, waitForDaemonRules } from "@/shared/stores/sync-rules";

const App: React.FC = () => {
  useGlobalShortcut();
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
  const setBreakpointRules = useBreakpointStore((s) => s.setRules);
  const addPendingBreakpoint = useBreakpointStore((s) => s.addPendingBreakpoint);
  const setHostMappings = useHostMappingStore((s) => s.setMappings);

  // 앱 시작 시 프록시 초기화 → 데몬 규칙 수신 대기 → 저장된 규칙 동기화
  useEffect(() => {
    initializeProxy().then(() => waitForDaemonRules().then(() => syncToProxy()));
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
      updateDaemonRules(rules);
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

  // Breakpoint 이벤트 수신
  useEffect(() => {
    const unlistenRules = listen<BreakpointRule[]>("breakpoint_rules_updated", (event) => {
      setBreakpointRules(event.payload);
    });
    const unlistenHit = listen<PendingBreakpoint>("breakpoint_hit", (event) => {
      addPendingBreakpoint(event.payload);
    });

    return () => {
      unlistenRules.then((f) => f());
      unlistenHit.then((f) => f());
    };
  }, [setBreakpointRules, addPendingBreakpoint]);

  // 데몬에서 호스트 매핑 변경 수신
  useEffect(() => {
    const unlisten = listen<HostMapping[]>("host_mappings_updated", (event) => {
      setHostMappings(event.payload);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [setHostMappings]);

  // 트레이 ↔ 메인 윈도우 양방향 동기화 (Tauri Store)
  const clearTransactions = useTransactionStore((s) => s.clearTransactions);
  const setConnected = useProxyStore((s) => s.setConnected);
  const transactionCount = useTransactionStore((s) => s.transactions.length);

  // 메인 → 트레이: 상태를 Store에 쓰기 (2초 디바운스)
  useEffect(() => {
    const timer = setTimeout(async () => {
      try {
        await trayStore.set("transactionCount", transactionCount);
        await trayStore.save();
      } catch {
        // 스토어 초기화 전이면 무시
      }
    }, 2000);
    return () => clearTimeout(timer);
  }, [transactionCount]);

  // 트레이 → 메인: Store 변경 감지로 상태 반영
  useEffect(() => {
    const unlistenPromise = trayStore.onChange((key, value) => {
      if (key === "proxyConnected" && typeof value === "boolean") {
        setConnected(value);
      }
      if (key === "clearSession" && typeof value === "number") {
        clearTransactions();
      }
    });

    return () => {
      unlistenPromise.then((f) => f());
    };
  }, [setConnected, clearTransactions]);

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
