import { create } from "zustand";

import type { SseEventInfo, SseConnection, SseConnectionStatus } from "@/entities/sse";

interface SseStoreState {
  events: SseEventInfo[];
  connections: Map<string, SseConnection>;
  selectedConnectionId: string | null;
  selectedEvent: SseEventInfo | null;

  addEvents: (events: SseEventInfo[]) => void;
  updateConnection: (
    connectionId: string,
    status: SseConnectionStatus,
    uri?: string,
    time?: number,
  ) => void;
  setSelectedConnectionId: (id: string | null) => void;
  setSelectedEvent: (event: SseEventInfo | null) => void;
  clearAll: () => void;
}

export const useSseStore = create<SseStoreState>()((set) => ({
  events: [],
  connections: new Map(),
  selectedConnectionId: null,
  selectedEvent: null,

  addEvents: (newEvents) =>
    set((state) => {
      const connections = new Map(state.connections);
      for (const event of newEvents) {
        if (!connections.has(event.connection_id)) {
          connections.set(event.connection_id, {
            id: event.connection_id,
            uri: event.connection_id,
            status: "connected",
            startTime: event.time,
            eventCount: 1,
          });
        } else {
          const conn = connections.get(event.connection_id)!;
          connections.set(event.connection_id, {
            ...conn,
            eventCount: conn.eventCount + 1,
          });
        }
      }

      return {
        events: state.events.concat(newEvents),
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
          eventCount: existing?.eventCount ?? 0,
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

  setSelectedConnectionId: (id) => set({ selectedConnectionId: id, selectedEvent: null }),

  setSelectedEvent: (event) => set({ selectedEvent: event }),

  clearAll: () => {
    sseEventBuffer.length = 0;
    if (sseFlushRafId) {
      cancelAnimationFrame(sseFlushRafId);
      sseFlushRafId = 0;
    }
    set({
      events: [],
      connections: new Map(),
      selectedConnectionId: null,
      selectedEvent: null,
    });
  },
}));

// --- rAF 버퍼링 ---

const sseEventBuffer: SseEventInfo[] = [];
let sseFlushRafId = 0;

function flushSseEvents() {
  sseFlushRafId = 0;
  if (sseEventBuffer.length === 0) return;
  const batch = sseEventBuffer.splice(0);
  useSseStore.getState().addEvents(batch);
}

export function bufferSseEvent(event: SseEventInfo) {
  sseEventBuffer.push(event);
  if (!sseFlushRafId) {
    sseFlushRafId = requestAnimationFrame(flushSseEvents);
  }
}
