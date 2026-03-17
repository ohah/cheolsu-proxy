/**
 * 나노초 타임스탬프를 HH:MM:SS.mmm 형식으로 변환
 */
export function formatTime(nanos: number): string {
  const ms = nanos / 1_000_000;
  const date = new Date(ms);
  const hms = date.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  const millis = String(date.getMilliseconds()).padStart(3, "0");
  return `${hms}.${millis}`;
}

/**
 * 나노초 타임스탬프를 로케일 전체 날짜/시간 형식으로 변환
 */
export function formatTimeFull(nanos: number): string {
  const ms = nanos / 1_000_000;
  return new Date(ms).toLocaleString();
}

/**
 * 요청/응답 나노초 타임스탬프 간의 지속 시간을 포맷
 */
export function formatDuration(requestNanos: number, responseNanos: number | null): string {
  if (responseNanos == null) return "-";
  const ms = (responseNanos - requestNanos) / 1_000_000;
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}
