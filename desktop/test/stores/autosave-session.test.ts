import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

// bun 테스트 환경에 localStorage가 없으므로 간단한 polyfill 제공
if (typeof globalThis.localStorage === "undefined") {
  const store = new Map<string, string>();
  globalThis.localStorage = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, value),
    removeItem: (key: string) => store.delete(key),
    clear: () => store.clear(),
    get length() {
      return store.size;
    },
    key: (index: number) => [...store.keys()][index] ?? null,
  };
}

// autosave_session / autoload_session invoke mock
const mockInvoke = mock((cmd: string, args?: Record<string, unknown>) => {
  if (cmd === "autosave_session") {
    return Promise.resolve();
  }
  if (cmd === "autoload_session") {
    return Promise.resolve(null);
  }
  return Promise.resolve();
});

mock.module("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

describe("autosave-session 설정 관리", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  test("기본값: 자동 저장 활성화", () => {
    // localStorage에 값이 없으면 기본 활성화
    const value = localStorage.getItem("autosave_session");
    expect(value).toBeNull();
    // 기본값 로직: !== "false" -> true
    expect(value !== "false").toBe(true);
  });

  test("자동 저장 비활성화 설정", () => {
    localStorage.setItem("autosave_session", "false");
    const enabled = localStorage.getItem("autosave_session") !== "false";
    expect(enabled).toBe(false);
  });

  test("자동 저장 활성화 설정", () => {
    localStorage.setItem("autosave_session", "true");
    const enabled = localStorage.getItem("autosave_session") !== "false";
    expect(enabled).toBe(true);
  });

  test("자동 저장 토글: 활성화 → 비활성화 → 활성화", () => {
    // 초기: 활성화 (기본값)
    expect(localStorage.getItem("autosave_session") !== "false").toBe(true);

    // 비활성화
    localStorage.setItem("autosave_session", JSON.stringify(false));
    expect(localStorage.getItem("autosave_session")).toBe("false");
    expect(localStorage.getItem("autosave_session") !== "false").toBe(false);

    // 다시 활성화
    localStorage.setItem("autosave_session", JSON.stringify(true));
    expect(localStorage.getItem("autosave_session")).toBe("true");
    expect(localStorage.getItem("autosave_session") !== "false").toBe(true);
  });
});

describe("autosave/autoload API", () => {
  beforeEach(() => {
    mockInvoke.mockClear();
  });

  test("autosave_session invoke 호출", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("autosave_session", { transactionsJson: "[]" });
    expect(mockInvoke).toHaveBeenCalledWith("autosave_session", { transactionsJson: "[]" });
  });

  test("autoload_session invoke 호출", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await invoke("autoload_session");
    expect(result).toBeNull();
    expect(mockInvoke).toHaveBeenCalledWith("autoload_session");
  });
});
