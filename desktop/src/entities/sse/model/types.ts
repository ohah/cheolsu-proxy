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

// Rust: #[serde(tag = "status")] enum — Connected/Disconnected 필드가 다름
export type SseConnectionEvent =
  | { status: "connected"; connection_id: string; uri: string; time: number }
  | { status: "disconnected"; connection_id: string; time: number };

export interface SseConnection {
  id: string;
  uri: string;
  status: SseConnectionStatus;
  startTime: number;
  endTime?: number;
  eventCount: number;
}
