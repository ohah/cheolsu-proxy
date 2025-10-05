// HTTP 메서드 타입
export type HttpMethod =
  | 'GET'
  | 'POST'
  | 'PUT'
  | 'DELETE'
  | 'PATCH'
  | 'HEAD'
  | 'OPTIONS'
  | 'CONNECT'
  | 'TRACE'
  | 'OTHERS';

// HTTP 상태 코드 타입
export type HttpStatusCode = number;

// 요청 페이로드 타입
export interface RequestPayload {
  headers?: Record<string, string>;
  data?: Record<string, unknown>;
  params?: Record<string, unknown> | string;
}

// 응답 페이로드 타입
export interface ResponsePayload {
  status: HttpStatusCode;
  headers?: Record<string, string>;
  data?: Record<string, unknown> | string;
}

export interface HttpRequest {
  method: string;
  uri: string;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
  id: string; // 고유 ID 추가
  content_type: string; // Content-Type 정보 추가
}

export interface HttpResponse {
  status: number;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array;
  time: number;
  content_type: string; // Content-Type 정보 추가
}

export interface HttpTransaction {
  request: HttpRequest | null;
  response: HttpResponse | null;
}

export type ProxyEventTuple = [HttpTransaction['request'], HttpTransaction['response']];
