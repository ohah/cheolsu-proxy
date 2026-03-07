import { create } from "zustand";

import type { WsMessageInfo, WsConnection, WsConnectionStatus } from "@/entities/websocket";

interface WebSocketStoreState {
  messages: WsMessageInfo[];
  connections: Map<string, WsConnection>;
  selectedConnectionId: string | null;
  selectedMessage: WsMessageInfo | null;

  addMessage: (message: WsMessageInfo) => void;
  updateConnection: (
    connectionId: string,
    status: WsConnectionStatus,
    uri?: string,
    time?: number,
  ) => void;
  setSelectedConnectionId: (id: string | null) => void;
  setSelectedMessage: (message: WsMessageInfo | null) => void;
  clearAll: () => void;
}

export const useWebSocketStore = create<WebSocketStoreState>()((set) => ({
  messages: [],
  connections: new Map(),
  selectedConnectionId: null,
  selectedMessage: null,

  addMessage: (message) =>
    set((state) => {
      // 연결이 없으면 자동 생성
      const connections = new Map(state.connections);
      if (!connections.has(message.connection_id)) {
        connections.set(message.connection_id, {
          id: message.connection_id,
          uri: message.connection_id,
          status: "connected",
          startTime: message.time,
          messageCount: 1,
        });
      } else {
        const conn = connections.get(message.connection_id)!;
        connections.set(message.connection_id, {
          ...conn,
          messageCount: conn.messageCount + 1,
        });
      }

      return {
        messages: [...state.messages, message],
        connections,
      };
    }),

  updateConnection: (connectionId, status, uri, time) =>
    set((state) => {
      const connections = new Map(state.connections);
      const existing = connections.get(connectionId);

      if (status === "connected") {
        connections.set(connectionId, {
          id: connectionId,
          uri: uri ?? connectionId,
          status: "connected",
          startTime: time ?? Date.now(),
          messageCount: existing?.messageCount ?? 0,
        });
      } else if (existing) {
        connections.set(connectionId, {
          ...existing,
          status: "disconnected",
          endTime: time ?? Date.now(),
        });
      }

      return { connections };
    }),

  setSelectedConnectionId: (id) => set({ selectedConnectionId: id, selectedMessage: null }),

  setSelectedMessage: (message) => set({ selectedMessage: message }),

  clearAll: () =>
    set({
      messages: [],
      connections: new Map(),
      selectedConnectionId: null,
      selectedMessage: null,
    }),
}));
