export interface HttpRequest {
  method: string;
  uri: string;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
  id: string; // 고유 ID 추가
}

export interface HttpResponse {
  status: number;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
}

export interface HttpTransaction {
  request: HttpRequest | null;
  response: HttpResponse | null;
}

export type ProxyEventTuple = [HttpTransaction['request'], HttpTransaction['response']];
