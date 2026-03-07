import { useMemo, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Trash2, Plug } from "lucide-react";

import { useWebSocketStore } from "@/shared/stores";
import { WsMessageTable, WsMessageDetail, WsConnectionList } from "@/widgets/websocket-messages";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup, Button } from "@/shared/ui";
import type { WsMessageInfo, WsConnection } from "@/entities/websocket";

export const WebSocketDashboard = () => {
  const { t } = useLingui();
  const messages = useWebSocketStore((s) => s.messages);
  const connections = useWebSocketStore((s) => s.connections);
  const selectedConnectionId = useWebSocketStore((s) => s.selectedConnectionId);
  const selectedMessage = useWebSocketStore((s) => s.selectedMessage);
  const setSelectedConnectionId = useWebSocketStore((s) => s.setSelectedConnectionId);
  const setSelectedMessage = useWebSocketStore((s) => s.setSelectedMessage);
  const clearAll = useWebSocketStore((s) => s.clearAll);

  const connectionList = useMemo<WsConnection[]>(
    () => Array.from(connections.values()).sort((a, b) => b.startTime - a.startTime),
    [connections],
  );

  const filteredMessages = useMemo(
    () =>
      selectedConnectionId
        ? messages.filter((m) => m.connection_id === selectedConnectionId)
        : messages,
    [messages, selectedConnectionId],
  );

  const messageCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const msg of messages) {
      counts.set(msg.connection_id, (counts.get(msg.connection_id) ?? 0) + 1);
    }
    return counts;
  }, [messages]);

  const handleSelectMessage = useCallback(
    (message: WsMessageInfo) => {
      setSelectedMessage(
        selectedMessage?.sequence === message.sequence &&
          selectedMessage?.connection_id === message.connection_id
          ? null
          : message,
      );
    },
    [selectedMessage, setSelectedMessage],
  );

  const handleCloseDetail = useCallback(() => {
    setSelectedMessage(null);
  }, [setSelectedMessage]);

  return (
    <div className="flex-1 flex flex-col h-full overflow-x-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-background flex-shrink-0">
        <div className="flex items-center gap-2">
          <Plug className="w-4 h-4 text-muted-foreground" />
          <h1 className="text-sm font-semibold">
            <Trans>WebSocket</Trans>
          </h1>
          <span className="text-xs text-muted-foreground">
            {connectionList.length} <Trans>connections</Trans> · {messages.length}{" "}
            <Trans>messages</Trans>
          </span>
        </div>
        <Button variant="ghost" size="sm" onClick={clearAll} title={t`Clear all`}>
          <Trash2 className="w-4 h-4" />
        </Button>
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-hidden">
        <ResizablePanelGroup orientation="horizontal" className="flex-1 flex bg-background">
          {/* Connection list */}
          <ResizablePanel
            id="ws-connections"
            className="h-full overflow-hidden"
            maxSize="30%"
            minSize="15%"
            collapsible
          >
            <WsConnectionList
              connections={connectionList}
              selectedConnectionId={selectedConnectionId}
              onSelectConnection={setSelectedConnectionId}
              messageCounts={messageCounts}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />

          {/* Message table */}
          <ResizablePanel id="ws-messages" className="flex flex-1 h-full overflow-hidden">
            <WsMessageTable
              messages={filteredMessages}
              selectedMessage={selectedMessage}
              onSelectMessage={handleSelectMessage}
            />
          </ResizablePanel>

          {selectedMessage && (
            <>
              <ResizableHandle withHandle />
              <ResizablePanel
                id="ws-detail"
                maxSize="50%"
                minSize="20%"
                className="h-full overflow-hidden"
              >
                <WsMessageDetail message={selectedMessage} onClose={handleCloseDetail} />
              </ResizablePanel>
            </>
          )}
        </ResizablePanelGroup>
      </div>
    </div>
  );
};
