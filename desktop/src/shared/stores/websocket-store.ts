import { create } from "zustand";

import type { WsMessageInfo, WsConnection, WsConnectionStatus } from "@/entities/websocket";

interface WebSocketStoreState {
  messages: WsMessageInfo[];
  connections: Map<string, WsConnection>;
  selectedConnectionId: string | null;
  selectedMessage: WsMessageInfo | null;

  addMessages: (messages: WsMessageInfo[]) => void;
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

  addMessages: (newMessages) =>
    set((state) => {
      const connections = new Map(state.connections);
      for (const message of newMessages) {
        if (!connections.has(message.connection_id)) {
          connections.set(message.connection_id, {
            id: message.connection_id,
            // connection_id는 이제 고유 ID(UUID)이므로 표시용으로는 메시지가 운반하는
            // 실제 URL을 우선 사용한다(Connected 이벤트가 유실돼 메시지로만 연결을
            // 만드는 경우에도 UUID가 노출되지 않도록).
            uri: message.url || message.connection_id,
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
      }

      return {
        messages: state.messages.concat(newMessages),
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

  clearAll: () => {
    wsMessageBuffer.length = 0;
    if (wsFlushRafId) {
      cancelAnimationFrame(wsFlushRafId);
      wsFlushRafId = 0;
    }
    set({
      messages: [],
      connections: new Map(),
      selectedConnectionId: null,
      selectedMessage: null,
    });
  },
}));

// --- rAF 버퍼링 ---

const wsMessageBuffer: WsMessageInfo[] = [];
let wsFlushRafId = 0;

function flushWsMessages() {
  wsFlushRafId = 0;
  if (wsMessageBuffer.length === 0) return;
  const batch = wsMessageBuffer.splice(0);
  useWebSocketStore.getState().addMessages(batch);
}

export function bufferWsMessage(message: WsMessageInfo) {
  wsMessageBuffer.push(message);
  if (!wsFlushRafId) {
    wsFlushRafId = requestAnimationFrame(flushWsMessages);
  }
}
