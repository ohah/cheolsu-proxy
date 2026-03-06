import { memo, useCallback, useRef, useEffect, useMemo } from "react";
import { ArrowUp, ArrowDown } from "lucide-react";
import { cn } from "@/shared/lib";
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
      () => (message.content_type === "mqtt" ? getMqttSummary(message.payload) : null),
      [message.content_type, message.payload],
    );

    return (
      <tr
        className={cn(
          "h-8 cursor-pointer border-b border-border/50 text-xs transition-colors",
          isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
          isSent ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400",
        )}
        onClick={onSelect}
      >
        <td className="px-2 w-8 text-center">
          <DirectionIcon className="w-3 h-3 inline-block" />
        </td>
        <td className="px-2 w-16">
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
        </td>
        <td className="px-2 w-20 text-right text-muted-foreground">{formatSize(message.size)}</td>
        <td className="px-2 flex-1 truncate max-w-0">
          <span className="text-foreground truncate block">
            {mqttSummary?.topic ?? message.payload}
          </span>
        </td>
        <td className="px-2 pr-4 w-24 text-right text-muted-foreground">
          {formatTime(message.time)}
        </td>
      </tr>
    );
  },
);
WsMessageRow.displayName = "WsMessageRow";

export const WsMessageTable = memo(
  ({ messages, selectedMessage, onSelectMessage }: WsMessageTableProps) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const shouldAutoScroll = useRef(true);

    const handleScroll = useCallback(() => {
      const el = containerRef.current;
      if (!el) return;
      const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
      shouldAutoScroll.current = distFromBottom < 50;
    }, []);

    useEffect(() => {
      if (shouldAutoScroll.current && containerRef.current) {
        containerRef.current.scrollTop = containerRef.current.scrollHeight;
      }
    }, [messages.length]);

    if (messages.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          WebSocket 메시지를 기다리는 중...
        </div>
      );
    }

    return (
      <div className="flex-1 flex flex-col overflow-hidden">
        <div className="flex-shrink-0 border-b border-border">
          <table className="w-full table-fixed">
            <thead>
              <tr className="h-7 text-xs text-muted-foreground font-medium bg-muted/30">
                <th className="px-2 w-8 text-center">Dir</th>
                <th className="px-2 w-16 text-left">Type</th>
                <th className="px-2 w-20 text-right">Size</th>
                <th className="px-2 text-left">Data</th>
                <th className="px-2 pr-4 w-24 text-right">Time</th>
              </tr>
            </thead>
          </table>
        </div>
        <div ref={containerRef} className="flex-1 overflow-y-auto" onScroll={handleScroll}>
          <table className="w-full table-fixed">
            <tbody>
              {messages.map((msg) => (
                <WsMessageRow
                  key={`${msg.connection_id}-${msg.sequence}`}
                  message={msg}
                  isSelected={
                    selectedMessage?.sequence === msg.sequence &&
                    selectedMessage?.connection_id === msg.connection_id
                  }
                  onSelect={() => onSelectMessage(msg)}
                />
              ))}
            </tbody>
          </table>
        </div>
      </div>
    );
  },
);
WsMessageTable.displayName = "WsMessageTable";
