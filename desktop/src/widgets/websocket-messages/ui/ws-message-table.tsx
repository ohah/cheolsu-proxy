import { memo, useCallback, useRef, useEffect, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ArrowUp, ArrowDown } from "lucide-react";
import { cn } from "@/shared/lib";
import { ScrollArea } from "@/shared/ui";
import { getMqttSummary } from "@/shared/lib/ws-content-view";
import type { WsMessageInfo } from "@/entities/websocket";

interface WsMessageTableProps {
  messages: WsMessageInfo[];
  selectedMessage: WsMessageInfo | null;
  onSelectMessage: (message: WsMessageInfo) => void;
}

function formatTime(nanos: number): string {
  const ms = nanos / 1_000_000;
  const date = new Date(ms);
  const hms = date.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  const millis = String(date.getMilliseconds()).padStart(3, "0");
  return `${hms}.${millis}`;
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function messageTypeLabel(type: string): string {
  switch (type) {
    case "text":
      return "Text";
    case "binary":
      return "Binary";
    case "ping":
      return "Ping";
    case "pong":
      return "Pong";
    case "close":
      return "Close";
    default:
      return type;
  }
}

const WsMessageRow = memo(
  ({
    message,
    isSelected,
    onSelect,
  }: {
    message: WsMessageInfo;
    isSelected: boolean;
    onSelect: () => void;
  }) => {
    const isSent = message.direction === "client_to_server";
    const DirectionIcon = isSent ? ArrowUp : ArrowDown;
    const mqttSummary = useMemo(
      () =>
        message.content_type === "mqtt"
          ? getMqttSummary(message.payload, message.mqtt_version)
          : null,
      [message.content_type, message.payload, message.mqtt_version],
    );

    return (
      <div
        className={cn(
          "h-8 flex items-center cursor-pointer border-b border-border/50 text-xs transition-colors",
          isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
          isSent ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400",
        )}
        onClick={onSelect}
      >
        <div className="px-2 w-8 text-center flex-shrink-0">
          <DirectionIcon className="w-3 h-3 inline-block" />
        </div>
        <div className="px-2 w-16 flex-shrink-0">
          {mqttSummary ? (
            <span className="inline-block px-1.5 py-0.5 rounded text-[10px] font-medium bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-300">
              {mqttSummary.packetType}
            </span>
          ) : (
            <>
              <span
                className={cn(
                  "inline-block px-1.5 py-0.5 rounded text-[10px] font-medium",
                  message.message_type === "text" &&
                    "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
                  message.message_type === "binary" &&
                    "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300",
                  message.message_type === "ping" &&
                    "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300",
                  message.message_type === "pong" &&
                    "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300",
                  message.message_type === "close" &&
                    "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300",
                )}
              >
                {messageTypeLabel(message.message_type)}
              </span>
              {message.content_type === "socket_io" && (
                <span className="inline-block ml-1 px-1 py-0.5 rounded text-[9px] font-medium bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-300">
                  SIO
                </span>
              )}
            </>
          )}
        </div>
        <div className="px-2 w-20 text-right text-muted-foreground flex-shrink-0">
          {formatSize(message.size)}
        </div>
        <div className="px-2 flex-1 truncate min-w-0">
          <span className="text-foreground truncate block">
            {mqttSummary?.topic ?? message.payload}
          </span>
        </div>
        <div className="px-2 pr-4 w-24 text-right text-muted-foreground flex-shrink-0">
          {formatTime(message.time)}
        </div>
      </div>
    );
  },
);
WsMessageRow.displayName = "WsMessageRow";

const ROW_HEIGHT = 32; // h-8

export const WsMessageTable = memo(
  ({ messages, selectedMessage, onSelectMessage }: WsMessageTableProps) => {
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
      count: messages.length,
      getScrollElement,
      estimateSize,
      overscan: 20,
    });

    useEffect(() => {
      if (shouldAutoScroll.current && messages.length > 0) {
        virtualizer.scrollToIndex(messages.length - 1, { align: "end" });
      }
    }, [messages.length]);

    const selectHandlers = useMemo(
      () => messages.map((msg) => () => onSelectMessage(msg)),
      [messages, onSelectMessage],
    );

    if (messages.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          <Trans>Waiting for WebSocket messages...</Trans>
        </div>
      );
    }

    return (
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        <div className="flex-shrink-0 border-b border-border">
          <div className="h-7 flex items-center text-xs text-muted-foreground font-medium bg-muted/30">
            <div className="px-2 w-8 text-center flex-shrink-0">{t`Dir`}</div>
            <div className="px-2 w-16 text-left flex-shrink-0">{t`Type`}</div>
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
              const msg = messages[virtualItem.index];
              return (
                <div
                  key={`${msg.connection_id}-${msg.sequence}`}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <WsMessageRow
                    message={msg}
                    isSelected={
                      selectedMessage?.sequence === msg.sequence &&
                      selectedMessage?.connection_id === msg.connection_id
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
WsMessageTable.displayName = "WsMessageTable";
