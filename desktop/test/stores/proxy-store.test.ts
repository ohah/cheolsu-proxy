import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

let shouldStartFail = false;

const mockInvoke = mock((cmd: string, _args?: unknown) => {
  if (cmd === "start_proxy_v2" && shouldStartFail) {
    return Promise.reject(new Error("연결 실패"));
  }
  return Promise.resolve();
});

mock.module("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

async function getStore() {
  const { useProxyStore } = await import("../../src/shared/stores/proxy-store");
  return useProxyStore;
}

describe("proxy-store", () => {
  let store: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    shouldStartFail = false;
    store = await getStore();
    store.setState({
      isConnected: false,
      isInitialized: false,
    });
  });

  test("초기 상태", () => {
    const state = store.getState();
    expect(state.isConnected).toBe(false);
    expect(state.isInitialized).toBe(false);
  });

  test("initializeProxy: 초기화 성공 시 isConnected 설정", async () => {
    await store.getState().initializeProxy(8100);
    const state = store.getState();
    expect(state.isInitialized).toBe(true);
    expect(state.isConnected).toBe(true);
  });

  test("initializeProxy: 이미 초기화된 상태면 중복 실행 방지", async () => {
    await store.getState().initializeProxy(8100);
    const callCountAfterFirst = mockInvoke.mock.calls.length;

    // 두 번째 호출은 스킵되어야 함
    await store.getState().initializeProxy(8100);
    expect(mockInvoke.mock.calls.length).toBe(callCountAfterFirst);
  });

  test("initializeProxy: startProxyV2 실패 시 isInitialized 리셋", async () => {
    shouldStartFail = true;

    await store.getState().initializeProxy(8100);
    const state = store.getState();
    expect(state.isConnected).toBe(false);
    expect(state.isInitialized).toBe(false);
  });

  test("setConnected: 연결 상태 변경", () => {
    store.getState().setConnected(true);
    expect(store.getState().isConnected).toBe(true);
    store.getState().setConnected(false);
    expect(store.getState().isConnected).toBe(false);
  });

  test("initializeProxy: 커스텀 포트로 프록시 시작", async () => {
    await store.getState().initializeProxy(9090);
    expect(store.getState().isConnected).toBe(true);
    // start_proxy_v2가 커스텀 포트로 호출되었는지 확인
    const lastCall = mockInvoke.mock.calls[mockInvoke.mock.calls.length - 1];
    expect(lastCall[0]).toBe("start_proxy_v2");
    expect(lastCall[1]).toEqual({ addr: "127.0.0.1:9090" });
  });
});
