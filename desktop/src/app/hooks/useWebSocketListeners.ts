import { useEffect } from "react";
import { useWebSocketStore, useTransactionStore } from "@/shared/stores";
import { bufferWsMessage } from "@/shared/stores/websocket-store";
import { listen } from "@tauri-apps/api/event";
import type { WsMessageInfo, WsConnectionEvent } from "@/entities/websocket";

/**
 * WebSocket 메시지 및 연결 이벤트 수신을 담당하는 훅
 */
export function useWebSocketListeners() {
  const updateWsConnection = useWebSocketStore((s) => s.updateConnection);
  const paused = useTransactionStore((s) => s.paused);

  // WebSocket 메시지 이벤트 수신
  useEffect(() => {
    if (paused) return;

    const unlistenMsg = listen<WsMessageInfo>("ws_message", (event) => {
      bufferWsMessage(event.payload);
    });
    const unlistenConn = listen<WsConnectionEvent>("ws_connection", (event) => {
      const { connection_id, status, uri, time } = event.payload;
      updateWsConnection(connection_id, status, uri, time);
    });

    return () => {
      unlistenMsg.then((f) => f());
      unlistenConn.then((f) => f());
    };
  }, [updateWsConnection, paused]);
}
