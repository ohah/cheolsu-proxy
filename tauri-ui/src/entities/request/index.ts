// Request 엔티티 타입 정의
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

export interface RequestInfo {
  request: ProxiedRequest | null;
  response: ProxiedResponse | null;
}
