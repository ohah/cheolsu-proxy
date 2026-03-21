import { describe, test, expect, mock, beforeEach } from "bun:test";
import { createDebouncedSync } from "../../src/shared/stores/create-debounced-sync";

describe("createDebouncedSync", () => {
  beforeEach(() => {
    // 타이머 자동 실행을 위해 짧은 delay 사용
  });

  test("함수가 debounce 후 실행된다", async () => {
    const fn = mock(() => Promise.resolve());
    const debouncedSync = createDebouncedSync(10);

    debouncedSync(fn);

    // 즉시 호출되지 않음
    expect(fn).not.toHaveBeenCalled();

    // debounce 시간 경과 후 호출됨
    await new Promise((r) => setTimeout(r, 20));
    expect(fn).toHaveBeenCalledTimes(1);
  });

  test("연속 호출 시 마지막 호출만 실행된다", async () => {
    const fn1 = mock(() => Promise.resolve());
    const fn2 = mock(() => Promise.resolve());
    const fn3 = mock(() => Promise.resolve());
    const debouncedSync = createDebouncedSync(10);

    debouncedSync(fn1);
    debouncedSync(fn2);
    debouncedSync(fn3);

    await new Promise((r) => setTimeout(r, 20));

    expect(fn1).not.toHaveBeenCalled();
    expect(fn2).not.toHaveBeenCalled();
    expect(fn3).toHaveBeenCalledTimes(1);
  });

  test("debounce 시간 경과 후 새 호출은 별도로 실행된다", async () => {
    const fn1 = mock(() => Promise.resolve());
    const fn2 = mock(() => Promise.resolve());
    const debouncedSync = createDebouncedSync(10);

    debouncedSync(fn1);
    await new Promise((r) => setTimeout(r, 20));
    expect(fn1).toHaveBeenCalledTimes(1);

    debouncedSync(fn2);
    await new Promise((r) => setTimeout(r, 20));
    expect(fn2).toHaveBeenCalledTimes(1);
  });

  test("각 인스턴스는 독립적인 타이머를 가진다", async () => {
    const fn1 = mock(() => Promise.resolve());
    const fn2 = mock(() => Promise.resolve());
    const debounce1 = createDebouncedSync(10);
    const debounce2 = createDebouncedSync(10);

    debounce1(fn1);
    debounce2(fn2);

    await new Promise((r) => setTimeout(r, 20));

    // 둘 다 독립적으로 실행됨
    expect(fn1).toHaveBeenCalledTimes(1);
    expect(fn2).toHaveBeenCalledTimes(1);
  });

  test("기본 delay는 300ms", async () => {
    const fn = mock(() => Promise.resolve());
    const debouncedSync = createDebouncedSync();

    debouncedSync(fn);

    // 100ms 후에는 아직 실행 안 됨
    await new Promise((r) => setTimeout(r, 100));
    expect(fn).not.toHaveBeenCalled();

    // 350ms 후에는 실행됨
    await new Promise((r) => setTimeout(r, 250));
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
