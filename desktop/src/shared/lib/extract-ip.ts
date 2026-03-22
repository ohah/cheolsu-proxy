/**
 * "IP:port" 형식에서 IP 부분만 추출
 * IPv6 "[::1]:54321" → "::1", IPv4 "192.168.1.1:54321" → "192.168.1.1"
 */
export function extractIp(clientAddr: string): string {
  if (clientAddr.startsWith("[")) {
    return clientAddr.replace(/^\[(.+)\]:\d+$/, "$1");
  }
  return clientAddr.replace(/:\d+$/, "");
}
