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

// 세션 스토어 타입
export interface SessionStore {
  id: string;
  url: string;
  method: HttpMethod;
  request?: RequestPayload;
  response?: ResponsePayload;
}

// 세션 스토어 상태 타입
export interface SessionStoreState {
  sessions: SessionStore[];
  setSessions: (sessions: SessionStore[]) => void;
  addSession: (session: SessionStore) => void;
  updateSession: (session: SessionStore) => void;
  deleteSession: (id: string) => void;
  deleteSessionByUrl: (url: string) => void;
}
