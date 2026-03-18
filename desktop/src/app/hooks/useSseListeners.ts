import { useEffect } from "react";
import { useSseStore, useTransactionStore } from "@/shared/stores";
import { bufferSseEvent } from "@/shared/stores/sse-store";
import { listen } from "@tauri-apps/api/event";
import type { SseEventInfo, SseConnectionEvent } from "@/entities/sse";

/**
 * SSE 이벤트 및 연결 이벤트 수신을 담당하는 훅
 */
export function useSseListeners() {
  const updateSseConnection = useSseStore((s) => s.updateConnection);
  const paused = useTransactionStore((s) => s.paused);

  // SSE 이벤트 수신
  useEffect(() => {
    if (paused) return;

    const unlistenEvent = listen<SseEventInfo>("sse_event", (event) => {
      bufferSseEvent(event.payload);
    });
    const unlistenConn = listen<SseConnectionEvent>("sse_connection", (event) => {
      const payload = event.payload;
      if (payload.status === "connected") {
        updateSseConnection(payload.connection_id, "connected", payload.uri, payload.time);
      } else {
        updateSseConnection(payload.connection_id, "disconnected", undefined, payload.time);
      }
    });

    return () => {
      unlistenEvent.then((f) => f());
      unlistenConn.then((f) => f());
    };
  }, [updateSseConnection, paused]);
}
