export type WsDirection = "client_to_server" | "server_to_client";

export type WsMessageType = "text" | "binary" | "ping" | "pong" | "close";

export type WsContentType = "plain" | "socket_io" | "mqtt";

export interface WsMessageInfo {
  connection_id: string;
  // 표시용 연결 URL(스킴 포함). connection_id가 고유 ID(UUID)로 바뀐 뒤, 메시지만으로
  // 연결을 표시할 때 UUID 대신 실제 URL을 쓰기 위해 백엔드가 함께 보낸다.
  // 과거 데이터에는 없을 수 있어 optional.
  url?: string;
  sequence: number;
  direction: WsDirection;
  message_type: WsMessageType;
  payload: string;
  size: number;
  time: number;
  is_binary: boolean;
  content_type?: WsContentType;
  mqtt_version?: number;
}

export type WsConnectionStatus = "connected" | "disconnected";

export interface WsConnectionEvent {
  status: WsConnectionStatus;
  connection_id: string;
  uri?: string;
  time: number;
}

export interface WsConnection {
  id: string;
  uri: string;
  status: WsConnectionStatus;
  startTime: number;
  endTime?: number;
  messageCount: number;
}
