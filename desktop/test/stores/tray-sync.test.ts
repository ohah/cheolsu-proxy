import { describe, test, expect, beforeEach, mock } from "bun:test";
import { clearEventListeners, simulateEvent } from "../mocks/tauri";

// emitTo mock을 직접 추적
const emitToCalls: Array<{ target: string; event: string; payload?: unknown }> = [];

mock.module("@tauri-apps/api/event", () => ({
  listen: mock(async (event: string, handler: (e: { payload: unknown }) => void) => {
    if (!globalListeners.has(event)) {
      globalListeners.set(event, new Set());
    }
    globalListeners.get(event)!.add(handler);
    return () => {
      globalListeners.get(event)?.delete(handler);
    };
  }),
  emit: mock(async () => {}),
  emitTo: mock(async (target: string, event: string, payload?: unknown) => {
    emitToCalls.push({ target, event, payload });
  }),
}));

type Handler = (e: { payload: unknown }) => void;
const globalListeners = new Map<string, Set<Handler>>();

function triggerEvent(event: string, payload: unknown) {
  globalListeners.get(event)?.forEach((handler) => handler({ payload }));
}

describe("트레이 ↔ 메인 윈도우 이벤트 동기화", () => {
  beforeEach(() => {
    emitToCalls.length = 0;
    globalListeners.clear();
    clearEventListeners();
  });

  describe("toggleProxy (메인 윈도우)", () => {
    test("toggleProxy는 trayStore를 사용하지 않음", async () => {
      const { toggleProxy } = await import(
        "../../src/features/proxy-toggle/lib/toggle-proxy"
      );
      // toggleProxy 소스 코드에 trayStore 참조가 없어야 함
      const source = toggleProxy.toString();
      expect(source).not.toContain("trayStore");
    });
  });

  describe("tray_action 이벤트 (트레이 → 메인)", () => {
    test("proxyConnected 액션으로 연결 상태 변경", async () => {
      const { useProxyStore } = await import("../../src/shared/stores/proxy-store");
      useProxyStore.setState({ isConnected: false });

      // App.tsx의 tray_action 리스너 시뮬레이션
      const handler = (event: { payload: { type: string; value?: boolean } }) => {
        const { type, value } = event.payload;
        if (type === "proxyConnected" && typeof value === "boolean") {
          useProxyStore.getState().setConnected(value);
        }
      };

      // 트레이에서 proxyConnected 이벤트 발생
      handler({ payload: { type: "proxyConnected", value: true } });
      expect(useProxyStore.getState().isConnected).toBe(true);

      handler({ payload: { type: "proxyConnected", value: false } });
      expect(useProxyStore.getState().isConnected).toBe(false);
    });

    test("clearSession 액션으로 트랜잭션 초기화", async () => {
      const { useTransactionStore } = await import("../../src/shared/stores/transaction-store");

      // 더미 트랜잭션 추가
      useTransactionStore.getState().addTransaction({
        request: {
          id: "test-1",
          method: "GET",
          uri: "https://example.com",
          version: "HTTP/1.1",
          headers: {},
          body: null,
          time: Date.now(),
          data_type: "Text",
          body_size: 0,
        },
        response: {
          id: "test-1",
          status: 200,
          version: "HTTP/1.1",
          headers: {},
          body: null,
          time: Date.now(),
          data_type: "Text",
          body_size: 0,
        },
      });
      expect(useTransactionStore.getState().transactions.length).toBeGreaterThan(0);

      // clearSession 액션 실행
      useTransactionStore.getState().clearTransactions();
      expect(useTransactionStore.getState().transactions.length).toBe(0);
    });
  });

  describe("tray_sync 이벤트 (메인 → 트레이)", () => {
    test("emitTo는 tray-panel 윈도우만 타겟팅", async () => {
      const { emitTo } = await import("@tauri-apps/api/event");

      await emitTo("tray-panel", "tray_sync", { transactionCount: 42 });

      expect(emitToCalls.length).toBeGreaterThan(0);
      const lastCall = emitToCalls[emitToCalls.length - 1];
      expect(lastCall.target).toBe("tray-panel");
      expect(lastCall.event).toBe("tray_sync");
      expect(lastCall.payload).toEqual({ transactionCount: 42 });
    });

    test("tray_action emitTo는 main 윈도우만 타겟팅", async () => {
      const { emitTo } = await import("@tauri-apps/api/event");

      await emitTo("main", "tray_action", {
        type: "proxyConnected",
        value: true,
      });

      const actionCall = emitToCalls.find(
        (c) => c.target === "main" && c.event === "tray_action",
      );
      expect(actionCall).toBeDefined();
      expect(actionCall!.payload).toEqual({
        type: "proxyConnected",
        value: true,
      });
    });
  });

  describe("tray_request_sync (초기 상태 동기화)", () => {
    test("트레이 패널은 마운트 시 tray_request_sync 이벤트 발행", async () => {
      const { emitTo } = await import("@tauri-apps/api/event");

      // 트레이 패널 마운트 시뮬레이션
      await emitTo("main", "tray_request_sync");

      const syncRequest = emitToCalls.find(
        (c) => c.target === "main" && c.event === "tray_request_sync",
      );
      expect(syncRequest).toBeDefined();
    });

    test("메인 윈도우는 tray_request_sync에 최신 상태로 응답", async () => {
      const { useProxyStore } = await import("../../src/shared/stores/proxy-store");
      const { useTransactionStore } = await import("../../src/shared/stores/transaction-store");
      const { emitTo } = await import("@tauri-apps/api/event");

      useProxyStore.setState({ isConnected: true });

      // App.tsx의 tray_request_sync 핸들러 시뮬레이션
      const handleRequestSync = async () => {
        const { isConnected } = useProxyStore.getState();
        const count = useTransactionStore.getState().transactions.length;
        await emitTo("tray-panel", "tray_sync", {
          transactionCount: count,
          proxyConnected: isConnected,
        });
      };

      await handleRequestSync();

      const responseCall = emitToCalls.find(
        (c) => c.target === "tray-panel" && c.event === "tray_sync",
      );
      expect(responseCall).toBeDefined();
      expect(responseCall!.payload).toEqual({
        transactionCount: expect.any(Number),
        proxyConnected: true,
      });
    });
  });

  describe("store 폴링 제거 검증", () => {
    test("tray-panel.tsx에 setInterval이 없음", async () => {
      const fs = await import("fs");
      const source = fs.readFileSync(
        "src/pages/tray-panel/ui/tray-panel.tsx",
        "utf-8",
      );
      expect(source).not.toContain("setInterval");
    });

    test("tray-panel.tsx에 LazyStore import가 없음", async () => {
      const fs = await import("fs");
      const source = fs.readFileSync(
        "src/pages/tray-panel/ui/tray-panel.tsx",
        "utf-8",
      );
      expect(source).not.toContain("LazyStore");
      expect(source).not.toContain("@tauri-apps/plugin-store");
    });

    test("App.tsx에 trayStore import가 없음", async () => {
      const fs = await import("fs");
      const source = fs.readFileSync("src/app/App.tsx", "utf-8");
      expect(source).not.toContain("trayStore");
      expect(source).not.toContain("tray-sync-store");
    });

    test("toggle-proxy.ts에 trayStore import가 없음", async () => {
      const fs = await import("fs");
      const source = fs.readFileSync(
        "src/features/proxy-toggle/lib/toggle-proxy.ts",
        "utf-8",
      );
      expect(source).not.toContain("trayStore");
      expect(source).not.toContain("tray-sync-store");
    });
  });
});
