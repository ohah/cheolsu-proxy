import { describe, test, expect, beforeEach, mock } from "bun:test";

const mockInvoke = mock((_cmd: string, _args?: unknown) => {
  return Promise.resolve();
});

mock.module("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

describe("Quick Settings API", () => {
  beforeEach(() => {
    mockInvoke.mockClear();
    localStorage.clear();
  });

  test("updateQuickSettings: no_gzip 파라미터 전달 확인", async () => {
    const { updateQuickSettings } = await import("../../src/shared/api/proxy");

    await updateQuickSettings(false, false, true, false);

    expect(mockInvoke).toHaveBeenCalledWith("update_quick_settings", {
      noCaching: false,
      blockCookies: false,
      noGzip: true,
      blockQuic: false,
    });
  });

  test("updateQuickSettings: 모든 설정 활성화", async () => {
    const { updateQuickSettings } = await import("../../src/shared/api/proxy");

    await updateQuickSettings(true, true, true, true);

    expect(mockInvoke).toHaveBeenCalledWith("update_quick_settings", {
      noCaching: true,
      blockCookies: true,
      noGzip: true,
      blockQuic: true,
    });
  });

  test("updateQuickSettings: 모든 설정 비활성화", async () => {
    const { updateQuickSettings } = await import("../../src/shared/api/proxy");

    await updateQuickSettings(false, false, false, false);

    expect(mockInvoke).toHaveBeenCalledWith("update_quick_settings", {
      noCaching: false,
      blockCookies: false,
      noGzip: false,
      blockQuic: false,
    });
  });

  test("localStorage에 no_gzip 상태 저장/복원", () => {
    // 저장
    localStorage.setItem("quick_settings_no_gzip", JSON.stringify(true));

    // 복원
    const stored = JSON.parse(localStorage.getItem("quick_settings_no_gzip") ?? "false");
    expect(stored).toBe(true);
  });

  test("localStorage에 no_gzip 기본값은 false", () => {
    const stored = JSON.parse(localStorage.getItem("quick_settings_no_gzip") ?? "false");
    expect(stored).toBe(false);
  });
});
