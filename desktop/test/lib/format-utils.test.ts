import { describe, test, expect } from "bun:test";

import { formatBytes } from "../../src/shared/lib/format-bytes";
import { formatDuration } from "../../src/shared/lib/format-time";

describe("formatBytes", () => {
  test("0 바이트", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  test("바이트 단위", () => {
    expect(formatBytes(500)).toBe("500 B");
  });

  test("킬로바이트 단위", () => {
    expect(formatBytes(1024)).toBe("1 KB");
  });

  test("소수점 표시", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  test("메가바이트 단위", () => {
    expect(formatBytes(1048576)).toBe("1 MB");
  });

  test("기가바이트 단위", () => {
    expect(formatBytes(1073741824)).toBe("1 GB");
  });

  test("테라바이트 단위", () => {
    expect(formatBytes(1099511627776)).toBe("1 TB");
  });

  test("불필요한 소수점 제거", () => {
    expect(formatBytes(2048)).toBe("2 KB");
  });
});

describe("formatDuration", () => {
  test("응답 없으면 '-' 반환", () => {
    expect(formatDuration(1000000000, null)).toBe("-");
  });

  test("1ms 미만", () => {
    expect(formatDuration(0, 500000)).toBe("<1ms"); // 0.5ms
  });

  test("밀리초 단위", () => {
    expect(formatDuration(0, 150000000)).toBe("150ms"); // 150ms
  });

  test("초 단위", () => {
    expect(formatDuration(0, 2500000000)).toBe("2.5s"); // 2500ms
  });

  test("정확히 1초", () => {
    expect(formatDuration(0, 1000000000)).toBe("1.0s");
  });

  test("999ms는 밀리초 단위", () => {
    expect(formatDuration(0, 999000000)).toBe("999ms");
  });
});
