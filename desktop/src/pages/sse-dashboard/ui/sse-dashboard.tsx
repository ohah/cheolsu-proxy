import { useMemo, useCallback, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { Trash2, Radio } from "lucide-react";

import { useSseStore } from "@/shared/stores";
import { SseMessageTable, SseMessageDetail, SseConnectionList } from "@/widgets/sse-messages";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  Button,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { useDefaultLayout, type ImperativePanelHandle } from "react-resizable-panels";
import type { SseEventInfo, SseConnection } from "@/entities/sse";

export const SseDashboard = () => {
  const events = useSseStore((s) => s.events);
  const connections = useSseStore((s) => s.connections);
  const selectedConnectionId = useSseStore((s) => s.selectedConnectionId);
  const selectedEvent = useSseStore((s) => s.selectedEvent);
  const setSelectedConnectionId = useSseStore((s) => s.setSelectedConnectionId);
  const setSelectedEvent = useSseStore((s) => s.setSelectedEvent);
  const clearAll = useSseStore((s) => s.clearAll);

  const connectionList = useMemo<SseConnection[]>(
    () => Array.from(connections.values()).sort((a, b) => b.startTime - a.startTime),
    [connections],
  );

  const filteredEvents = useMemo(
    () =>
      selectedConnectionId
        ? events.filter((e) => e.connection_id === selectedConnectionId)
        : events,
    [events, selectedConnectionId],
  );

  const eventCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const evt of events) {
      counts.set(evt.connection_id, (counts.get(evt.connection_id) ?? 0) + 1);
    }
    return counts;
  }, [events]);

  const handleSelectEvent = useCallback(
    (event: SseEventInfo) => {
      setSelectedEvent(
        selectedEvent?.sequence === event.sequence &&
          selectedEvent?.connection_id === event.connection_id
          ? null
          : event,
      );
    },
    [selectedEvent, setSelectedEvent],
  );

  const handleCloseDetail = useCallback(() => {
    setSelectedEvent(null);
  }, [setSelectedEvent]);

  const detailPanelRef = useRef<ImperativePanelHandle>(null);

  useEffect(() => {
    if (selectedEvent) {
      detailPanelRef.current?.expand();
    } else {
      detailPanelRef.current?.collapse();
    }
  }, [selectedEvent]);

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "sse-dashboard-layout",
    storage: localStorage,
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-x-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-background flex-shrink-0">
        <div className="flex items-center gap-2">
          <Radio className="w-4 h-4 text-muted-foreground" />
          <h1 className="text-sm font-semibold">
            <Trans>SSE</Trans>
          </h1>
          <span className="text-xs text-muted-foreground">
            {connectionList.length} <Trans>connections</Trans> · {events.length}{" "}
            <Trans>events</Trans>
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
              "sse-connections": 20,
              "sse-events": 80,
            }
          }
          onLayoutChanged={onLayoutChanged}
          className="flex-1 flex bg-background"
        >
          {/* Connection list */}
          <ResizablePanel
            id="sse-connections"
            className="h-full overflow-hidden"
            maxSize="30%"
            minSize="15%"
            collapsible
          >
            <SseConnectionList
              connections={connectionList}
              selectedConnectionId={selectedConnectionId}
              onSelectConnection={setSelectedConnectionId}
              eventCounts={eventCounts}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />

          {/* Event table */}
          <ResizablePanel id="sse-events" className="flex flex-1 h-full overflow-hidden">
            <SseMessageTable
              events={filteredEvents}
              selectedEvent={selectedEvent}
              onSelectEvent={handleSelectEvent}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />
          <ResizablePanel
            ref={detailPanelRef}
            id="sse-detail"
            maxSize="50%"
            minSize="20%"
            defaultSize="0%"
            collapsible
            className="h-full overflow-hidden"
          >
            {selectedEvent && (
              <SseMessageDetail event={selectedEvent} onClose={handleCloseDetail} />
            )}
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};
