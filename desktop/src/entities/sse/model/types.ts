export interface SseEventInfo {
  connection_id: string;
  sequence: number;
  event_type?: string;
  data: string;
  id?: string;
  retry?: number;
  size: number;
  time: number;
}

export type SseConnectionStatus = "connected" | "disconnected";

export interface SseConnectionEvent {
  status: SseConnectionStatus;
  connection_id: string;
  uri?: string;
  time: number;
}

export interface SseConnection {
  id: string;
  uri: string;
  status: SseConnectionStatus;
  startTime: number;
  endTime?: number;
  eventCount: number;
}
