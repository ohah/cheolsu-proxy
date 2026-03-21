import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

import { invoke } from "@tauri-apps/api/core";

async function getStore() {
  const { useSslProxyingStore } = await import("../../src/shared/stores/ssl-proxying-store");
  return useSslProxyingStore;
}

describe("ssl-proxying-store", () => {
  beforeEach(async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);
    store.setState({ defaultPassthroughEntries: [] });
    (invoke as ReturnType<typeof mock>).mockClear();
  });

  test("초기 상태는 빈 엔트리 목록", async () => {
    const store = await getStore();
    store.getState().setFromDaemon("blacklist", []);
    expect(store.getState().entries).toEqual([]);
    expect(store.getState().mode).toBe("blacklist");
  });

  describe("mutation-time sync", () => {
    test("addEntry 시 syncToProxy가 debounce로 호출된다", async () => {
      const store = await getStore();
      store.getState().addEntry({ pattern: "example.com", enabled: true });

      expect(store.getState().entries).toHaveLength(1);
      expect(store.getState().entries[0]).toEqual({
        pattern: "example.com",
        enabled: true,
      });
      // debounced — 즉시 호출되지는 않지만 invoke mock이 setTimeout 후 호출됨
    });

    test("removeEntry 시 엔트리가 제거된다", async () => {
      const store = await getStore();
      store.getState().setFromDaemon("blacklist", [
        { pattern: "example.com", enabled: true },
        { pattern: "*.api.io", enabled: true },
      ]);

      (invoke as ReturnType<typeof mock>).mockClear();
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

    test("setMode로 모드 변경", async () => {
      const store = await getStore();
      store.getState().setMode("whitelist");
      expect(store.getState().mode).toBe("whitelist");
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

  describe("setFromDaemon — sync 루프 방지", () => {
    test("setFromDaemon은 syncToProxy를 호출하지 않는다", async () => {
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
  });

  describe("defaultPassthrough", () => {
    test("setDefaultPassthroughFromDaemon은 sync를 호출하지 않는다", async () => {
      const store = await getStore();
      const entries = [
        { pattern: "accounts.google.com", enabled: true },
        { pattern: "github.com", enabled: false },
      ];

      (invoke as ReturnType<typeof mock>).mockClear();
      store.getState().setDefaultPassthroughFromDaemon(entries);

      expect(store.getState().defaultPassthroughEntries).toEqual(entries);
      expect(invoke).not.toHaveBeenCalled();
    });

    test("setDefaultPassthroughEntries는 state를 업데이트한다", async () => {
      const store = await getStore();
      const entries = [
        { pattern: "accounts.google.com", enabled: true },
        { pattern: "github.com", enabled: false },
      ];

      store.getState().setDefaultPassthroughEntries(entries);
      expect(store.getState().defaultPassthroughEntries).toEqual(entries);
    });
  });
});
