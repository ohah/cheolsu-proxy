import type { HttpMethod, RequestPayload, ResponsePayload } from '@/entities/proxy';

// 세션 스토어 타입
export interface SessionStore {
  id: string;
  url: string;
  isActive: boolean;
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
