import { useCallback, useEffect } from "react";
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
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ProxyEventTuple, HttpTransaction } from "@/entities/proxy";
import { autosaveSession, autoloadSession } from "@/shared/api/proxy";
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

  const setTransactions = useTransactionStore((s) => s.setTransactions);

  // 앱 시작 시 자동 저장된 세션 복원
  useEffect(() => {
    const autoSessionEnabled = localStorage.getItem("autosave_session") !== "false";
    if (!autoSessionEnabled) return;

    autoloadSession()
      .then((result) => {
        if (result && result.transactions_json) {
          try {
            const tuples = JSON.parse(result.transactions_json) as [unknown, unknown][];
            const loaded: HttpTransaction[] = tuples.map(([request, response]) => ({
              request,
              response,
            })) as HttpTransaction[];
            if (loaded.length > 0) {
              setTransactions(loaded);
              console.info(`자동 세션 복원 완료: ${loaded.length}개 트랜잭션`);
            }
          } catch (e) {
            console.error("자동 세션 복원 파싱 실패:", e);
          }
        }
      })
      .catch((e) => {
        console.error("자동 세션 복원 실패:", e);
      });
  }, [setTransactions]);

  // 자동 세션 저장 로직
  const performAutosave = useCallback(async () => {
    const autoSessionEnabled = localStorage.getItem("autosave_session") !== "false";
    if (!autoSessionEnabled) return;

    try {
      const currentTransactions = useTransactionStore.getState().transactions;
      if (currentTransactions.length === 0) return;

      if (currentTransactions.length >= 5000) {
        console.warn(
          `[autosave] 트랜잭션 수가 ${currentTransactions.length}개로 많습니다. JSON 직렬화 시 UI 블로킹이 발생할 수 있습니다.`,
        );
      }

      const tuples = currentTransactions.map((tx) => [tx.request, tx.response]);

      const t0 = performance.now();
      const transactionsJson = JSON.stringify(tuples);
      const t1 = performance.now();

      if (t1 - t0 > 100) {
        console.warn(
          `[autosave] JSON.stringify에 ${(t1 - t0).toFixed(1)}ms 소요 (트랜잭션 ${currentTransactions.length}개)`,
        );
      }

      await autosaveSession(transactionsJson);
      console.info(`자동 세션 저장 완료: ${currentTransactions.length}개 트랜잭션`);
    } catch (e) {
      console.error("자동 세션 저장 실패:", e);
    }
  }, []);

  // 윈도우 닫기(숨김) 시 자동 세션 저장
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async () => {
      await performAutosave();
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [performAutosave]);

  // 앱 완전 종료 시 자동 세션 저장 (트레이 메뉴 종료)
  useEffect(() => {
    const unlisten = listen("app_quit_requested", async () => {
      await performAutosave();
      // 저장 완료를 백엔드에 알려 즉시 종료할 수 있도록 함
      await emit("autosave_completed");
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [performAutosave]);

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
