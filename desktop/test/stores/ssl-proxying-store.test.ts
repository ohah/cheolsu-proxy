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
    // clearEntries를 호출하여 이전 테스트 상태 제거
    store.getState().setFromDaemon("blacklist", []);
    expect(store.getState().entries).toEqual([]);
    expect(store.getState().mode).toBe("blacklist");
  });

  test("addEntry로 엔트리 추가", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);

    store.getState().addEntry({ pattern: "example.com", enabled: true });
    expect(store.getState().entries).toHaveLength(1);
    expect(store.getState().entries[0]).toEqual({
      pattern: "example.com",
      enabled: true,
    });
  });

  test("removeEntry로 엔트리 삭제", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [
      { pattern: "example.com", enabled: true },
      { pattern: "*.api.io", enabled: true },
    ]);

    store.getState().removeEntry("example.com");
    expect(store.getState().entries).toHaveLength(1);
    expect(store.getState().entries[0].pattern).toBe("*.api.io");
  });

  test("toggleEntry로 엔트리 토글", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [{ pattern: "example.com", enabled: true }]);

    store.getState().toggleEntry("example.com");
    expect(store.getState().entries[0].enabled).toBe(false);

    store.getState().toggleEntry("example.com");
    expect(store.getState().entries[0].enabled).toBe(true);
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
    // setFromDaemon은 syncToProxy를 호출하지 않으므로 invoke가 호출되지 않아야 함
    expect(invoke).not.toHaveBeenCalled();
  });

  test("clearEntries로 전체 삭제", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", [
      { pattern: "a.com", enabled: true },
      { pattern: "b.com", enabled: true },
    ]);

    store.getState().clearEntries();
    expect(store.getState().entries).toHaveLength(0);
  });
});
