import { useMemo, useCallback, useState, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { Trash2, Plug } from "lucide-react";

import { WsReplayDialog } from "@/features/websocket-replay";

import { useWebSocketStore } from "@/shared/stores";
import { WsMessageTable, WsMessageDetail, WsConnectionList } from "@/widgets/websocket-messages";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  Button,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { useDefaultLayout, type PanelImperativeHandle } from "react-resizable-panels";
import type { WsMessageInfo, WsConnection } from "@/entities/websocket";

export const WebSocketDashboard = () => {
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

  const [replayMessage, setReplayMessage] = useState<WsMessageInfo | null>(null);

  const handleReplayRequest = useCallback((message: WsMessageInfo) => {
    setReplayMessage(message);
  }, []);

  const detailPanelRef = useRef<PanelImperativeHandle>(null);

  useEffect(() => {
    if (selectedMessage) {
      detailPanelRef.current?.expand();
    } else {
      detailPanelRef.current?.collapse();
    }
  }, [selectedMessage]);

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "websocket-dashboard-layout",
    storage: localStorage,
  });

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
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Button variant="ghost" size="sm" onClick={clearAll}>
              <Trash2 className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent sideOffset={4}>
            <Trans>Clear all</Trans>
          </TooltipContent>
        </Tooltip>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <ResizablePanelGroup
          orientation="horizontal"
          defaultLayout={
            defaultLayout ?? {
              "ws-connections": 20,
              "ws-messages": 80,
            }
          }
          onLayoutChanged={onLayoutChanged}
          className="flex-1 flex bg-background"
        >
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

          <ResizableHandle withHandle />
          <ResizablePanel
            panelRef={detailPanelRef}
            id="ws-detail"
            maxSize="50%"
            minSize="20%"
            defaultSize="0%"
            collapsible
            className="h-full overflow-hidden"
          >
            {selectedMessage && (
              <WsMessageDetail
                message={selectedMessage}
                onClose={handleCloseDetail}
                onReplayRequest={handleReplayRequest}
              />
            )}
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>

      {replayMessage && (
        <WsReplayDialog
          message={replayMessage}
          open={!!replayMessage}
          onOpenChange={(open) => {
            if (!open) setReplayMessage(null);
          }}
        />
      )}
    </div>
  );
};
