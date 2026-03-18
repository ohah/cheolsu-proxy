import { memo, useCallback, useRef, useEffect, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "@/shared/lib";
import { ScrollArea } from "@/shared/ui";
import { formatBytes } from "@/shared/lib/format-bytes";
import { formatTime } from "@/shared/lib/format-time";
import type { SseEventInfo } from "@/entities/sse";

interface SseMessageTableProps {
  events: SseEventInfo[];
  selectedEvent: SseEventInfo | null;
  onSelectEvent: (event: SseEventInfo) => void;
}

const SseEventRow = memo(
  ({
    event,
    isSelected,
    onSelect,
  }: {
    event: SseEventInfo;
    isSelected: boolean;
    onSelect: () => void;
  }) => {
    return (
      <div
        className={cn(
          "h-8 flex items-center cursor-pointer border-b border-border/50 text-xs transition-colors",
          isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
        )}
        onClick={onSelect}
      >
        <div className="px-2 w-20 flex-shrink-0">
          <span
            className={cn(
              "inline-block px-1.5 py-0.5 rounded text-[10px] font-medium",
              event.event_type
                ? "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300"
                : "bg-gray-100 text-gray-600 dark:bg-gray-800/30 dark:text-gray-400",
            )}
          >
            {event.event_type ?? "message"}
          </span>
        </div>
        <div className="px-2 w-20 text-right text-muted-foreground flex-shrink-0">
          {formatBytes(event.size)}
        </div>
        <div className="px-2 flex-1 truncate min-w-0">
          <span className="text-foreground truncate block">{event.data}</span>
        </div>
        <div className="px-2 pr-4 w-24 text-right text-muted-foreground flex-shrink-0">
          {formatTime(event.time)}
        </div>
      </div>
    );
  },
);
SseEventRow.displayName = "SseEventRow";

const ROW_HEIGHT = 32;

export const SseMessageTable = memo(
  ({ events, selectedEvent, onSelectEvent }: SseMessageTableProps) => {
    const { t } = useLingui();
    const viewportRef = useRef<HTMLDivElement>(null);
    const shouldAutoScroll = useRef(true);

    const handleScroll = useCallback(() => {
      const el = viewportRef.current;
      if (!el) return;
      const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
      shouldAutoScroll.current = distFromBottom < 50;
    }, []);

    const scrollAreaCallbackRef = useCallback(
      (node: HTMLDivElement | null) => {
        if (node) {
          const viewport = node.querySelector(
            '[data-slot="scroll-area-viewport"]',
          ) as HTMLDivElement;
          if (viewport) {
            viewportRef.current = viewport;
            viewport.addEventListener("scroll", handleScroll);
          }
        } else if (viewportRef.current) {
          viewportRef.current.removeEventListener("scroll", handleScroll);
          viewportRef.current = null;
        }
      },
      [handleScroll],
    );

    const getScrollElement = useCallback(() => viewportRef.current, []);
    const estimateSize = useCallback(() => ROW_HEIGHT, []);

    const virtualizer = useVirtualizer({
      count: events.length,
      getScrollElement,
      estimateSize,
      overscan: 20,
    });

    useEffect(() => {
      if (shouldAutoScroll.current && events.length > 0) {
        virtualizer.scrollToIndex(events.length - 1, { align: "end" });
      }
    }, [events.length]);

    const selectHandlers = useMemo(
      () => events.map((evt) => () => onSelectEvent(evt)),
      [events, onSelectEvent],
    );

    if (events.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          <Trans>Waiting for SSE events...</Trans>
        </div>
      );
    }

    return (
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        <div className="flex-shrink-0 border-b border-border">
          <div className="h-7 flex items-center text-xs text-muted-foreground font-medium bg-muted/30">
            <div className="px-2 w-20 text-left flex-shrink-0">{t`Event`}</div>
            <div className="px-2 w-20 text-right flex-shrink-0">{t`Size`}</div>
            <div className="px-2 flex-1 text-left min-w-0">{t`Data`}</div>
            <div className="px-2 pr-4 w-24 text-right flex-shrink-0">{t`Time`}</div>
          </div>
        </div>
        <ScrollArea ref={scrollAreaCallbackRef} className="flex-1 h-full min-h-0">
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const evt = events[virtualItem.index];
              return (
                <div
                  key={`${evt.connection_id}-${evt.sequence}`}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <SseEventRow
                    event={evt}
                    isSelected={
                      selectedEvent?.sequence === evt.sequence &&
                      selectedEvent?.connection_id === evt.connection_id
                    }
                    onSelect={selectHandlers[virtualItem.index]}
                  />
                </div>
              );
            })}
          </div>
        </ScrollArea>
      </div>
    );
  },
);
SseMessageTable.displayName = "SseMessageTable";
