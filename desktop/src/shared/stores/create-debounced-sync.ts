/**
 * 스토어별 debounced sync 함수를 생성합니다.
 * 연속 호출을 방지하여 마지막 호출만 실행됩니다.
 */
export function createDebouncedSync(delayMs = 300) {
  let timer: ReturnType<typeof setTimeout> | null = null;

  return (fn: () => Promise<void>) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      timer = null;
      fn();
    }, delayMs);
  };
}
