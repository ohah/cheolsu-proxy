const HOP_BY_HOP_HEADERS = [
  "host",
  "connection",
  "keep-alive",
  "proxy-authenticate",
  "proxy-authorization",
  "te",
  "trailers",
  "transfer-encoding",
  "upgrade",
];

/**
 * hop-by-hop 헤더를 제거한 새 객체를 반환합니다.
 * 대소문자 양쪽 모두 삭제합니다 (예: "host" / "Host").
 */
export function sanitizeHopByHopHeaders(
  headers: Record<string, string>,
): Record<string, string> {
  const result = { ...headers };
  for (const key of HOP_BY_HOP_HEADERS) {
    delete result[key];
    delete result[key.charAt(0).toUpperCase() + key.slice(1)];
  }
  return result;
}
