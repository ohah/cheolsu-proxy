import { describe, test, expect, beforeEach } from "bun:test";
import "../../test/mocks/tauri";

async function getStore() {
  const { useScriptStore } = await import(
    "../../src/shared/stores/script-store"
  );
  return useScriptStore;
}

describe("script-store", () => {
  let store: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    store = await getStore();
    store.setState({
      active: false,
      path: null,
      logs: [],
    });
  });

  test("초기 상태", () => {
    const state = store.getState();
    expect(state.active).toBe(false);
    expect(state.path).toBeNull();
    expect(state.logs).toEqual([]);
  });

  test("setStatus: 활성화 및 경로 설정", () => {
    store.getState().setStatus(true, "/path/to/script.ts");
    const state = store.getState();
    expect(state.active).toBe(true);
    expect(state.path).toBe("/path/to/script.ts");
  });

  test("setStatus: 비활성화", () => {
    store.getState().setStatus(true, "/path/to/script.ts");
    store.getState().setStatus(false, null);
    const state = store.getState();
    expect(state.active).toBe(false);
    expect(state.path).toBeNull();
  });

  test("addLog: 로그 추가", () => {
    store.getState().addLog("info", "스크립트 로드 완료");
    const logs = store.getState().logs;
    expect(logs).toHaveLength(1);
    expect(logs[0].level).toBe("info");
    expect(logs[0].message).toBe("스크립트 로드 완료");
    expect(typeof logs[0].time).toBe("number");
  });

  test("addLog: 여러 로그 추가", () => {
    store.getState().addLog("info", "시작");
    store.getState().addLog("warn", "경고");
    store.getState().addLog("error", "에러 발생");
    expect(store.getState().logs).toHaveLength(3);
    expect(store.getState().logs[0].level).toBe("info");
    expect(store.getState().logs[2].level).toBe("error");
  });

  test("addLog: 최대 5000개 유지", () => {
    // 5001개 로그 추가
    for (let i = 0; i < 5001; i++) {
      store.getState().addLog("info", `log-${i}`);
    }
    const logs = store.getState().logs;
    expect(logs).toHaveLength(5000);
    // 가장 오래된 로그(0번)가 삭제됨
    expect(logs[0].message).toBe("log-1");
    expect(logs[4999].message).toBe("log-5000");
  });

  test("clearLogs: 로그 초기화", () => {
    store.getState().addLog("info", "test");
    store.getState().addLog("error", "test2");
    store.getState().clearLogs();
    expect(store.getState().logs).toEqual([]);
  });

  test("clearLogs: 활성 상태는 유지", () => {
    store.getState().setStatus(true, "/script.ts");
    store.getState().addLog("info", "test");
    store.getState().clearLogs();
    expect(store.getState().active).toBe(true);
    expect(store.getState().path).toBe("/script.ts");
  });
});
