export type WsDirection = "client_to_server" | "server_to_client";

export type WsMessageType = "text" | "binary" | "ping" | "pong" | "close";

export type WsContentType = "plain" | "socket_io" | "mqtt";

export interface WsMessageInfo {
  connection_id: string;
  sequence: number;
  direction: WsDirection;
  message_type: WsMessageType;
  payload: string;
  size: number;
  time: number;
  is_binary: boolean;
  content_type?: WsContentType;
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
