import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

import { invoke } from "@tauri-apps/api/core";

async function getStore() {
  const { useSslProxyingStore } = await import("../../src/shared/stores/ssl-proxying-store");
  return useSslProxyingStore;
}

describe("ssl-proxying-store", () => {
  beforeEach(() => {
    (invoke as ReturnType<typeof mock>).mockClear();
  });

  test("초기 상태는 빈 엔트리 목록", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);
    expect(store.getState().entries).toEqual([]);
    expect(store.getState().mode).toBe("blacklist");
  });

  test("addEntry로 엔트리 추가 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);

    store.getState().addEntry({ pattern: "example.com", enabled: true });
    expect(store.getState().entries).toHaveLength(1);
    expect(store.getState().entries[0]).toEqual({
      pattern: "example.com",
      enabled: true,
    });
    // addEntry는 syncToProxy를 호출하지 않음
    expect(invoke).not.toHaveBeenCalled();
  });

  test("removeEntry로 엔트리 삭제 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [
      { pattern: "example.com", enabled: true },
      { pattern: "*.api.io", enabled: true },
    ]);

    (invoke as ReturnType<typeof mock>).mockClear();
    store.getState().removeEntry("example.com");
    expect(store.getState().entries).toHaveLength(1);
    expect(store.getState().entries[0].pattern).toBe("*.api.io");
    expect(invoke).not.toHaveBeenCalled();
  });

  test("toggleEntry로 엔트리 토글 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [{ pattern: "example.com", enabled: true }]);

    (invoke as ReturnType<typeof mock>).mockClear();
    store.getState().toggleEntry("example.com");
    expect(store.getState().entries[0].enabled).toBe(false);

    store.getState().toggleEntry("example.com");
    expect(store.getState().entries[0].enabled).toBe(true);
    expect(invoke).not.toHaveBeenCalled();
  });

  test("setFromDaemon으로 전체 목록 교체 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    const entries = [
      { pattern: "a.com", enabled: true },
      { pattern: "b.com", enabled: false },
    ];

    (invoke as ReturnType<typeof mock>).mockClear();
    store.getState().setFromDaemon("whitelist", entries);

    expect(store.getState().mode).toBe("whitelist");
    expect(store.getState().entries).toHaveLength(2);
    expect(store.getState().entries[0].pattern).toBe("a.com");
    expect(invoke).not.toHaveBeenCalled();
  });

  test("clearEntries로 전체 삭제 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [
      { pattern: "a.com", enabled: true },
      { pattern: "b.com", enabled: true },
    ]);

    (invoke as ReturnType<typeof mock>).mockClear();
    store.getState().clearEntries();
    expect(store.getState().entries).toHaveLength(0);
    expect(invoke).not.toHaveBeenCalled();
  });

  test("setMode로 모드 변경 (syncToProxy 호출 안 함)", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);

    (invoke as ReturnType<typeof mock>).mockClear();
    store.getState().setMode("whitelist");
    expect(store.getState().mode).toBe("whitelist");
    expect(invoke).not.toHaveBeenCalled();
  });
});
