import { DataType } from "./data-type";

// HTTP 메서드 타입
export type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "DELETE"
  | "PATCH"
  | "HEAD"
  | "OPTIONS"
  | "CONNECT"
  | "TRACE"
  | "OTHERS";

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
  body: Uint8Array | null; // 파일로 저장된 경우 null
  time: number;
  id: string; // 고유 ID 추가
  data_type: DataType; // 데이터 타입 정보 추가
  body_json?: unknown; // JSON 파싱된 데이터 (JSON 타입인 경우)
  file_path?: string; // body가 저장된 파일 경로
  body_size: number; // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
  client_addr?: string; // 클라이언트 IP 주소
  proxy_auth_user?: string; // 프록시 인증 사용자명
}

export interface TimingWaterfall {
  dns_lookup_ms?: number;
  tcp_connection_ms?: number;
  tls_handshake_ms?: number;
  ttfb_ms?: number;
  content_download_ms?: number;
  total_ms: number;
}

export interface HttpResponse {
  status: number;
  version: string;
  headers: Record<string, string>;
  body: Uint8Array | null; // 파일로 저장된 경우 null
  time: number;
  id: string; // ClientRequest의 id와 동일
  data_type: DataType; // 데이터 타입 정보 추가
  body_json?: unknown; // JSON 파싱된 데이터 (JSON 타입인 경우)
  file_path?: string; // body가 저장된 파일 경로
  body_size: number; // 실제 body 크기 (파일 저장 시에도 원본 크기 유지)
  timing?: TimingWaterfall; // 요청/응답 각 단계별 타이밍 정보
}

export interface HttpTransaction {
  request: HttpRequest | null;
  response: HttpResponse | null;
}

export type ProxyEventTuple = [HttpTransaction["request"], HttpTransaction["response"]];

// Re-export DataType for convenience
export { DataType } from "./data-type";
