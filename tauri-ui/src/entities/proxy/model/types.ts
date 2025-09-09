export interface ProxiedRequest {
  method: string;
  uri: string;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
  id: string; // 고유 ID 추가
}

export interface ProxiedResponse {
  status: number;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
}

export type ProxyEventRequestInfo = [ProxiedRequest | null, ProxiedResponse | null];

export interface RequestInfo {
  request: ProxiedRequest | null;
  response: ProxiedResponse | null;
}
