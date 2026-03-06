import { memo, useMemo } from "react";
import { ArrowUp, ArrowDown, X } from "lucide-react";
import { Button } from "@/shared/ui";
import { cn } from "@/shared/lib";
import type { WsMessageInfo } from "@/entities/websocket";

interface WsMessageDetailProps {
  message: WsMessageInfo;
  onClose: () => void;
}

function tryFormatJson(text: string): { formatted: string; isJson: boolean } {
  try {
    const parsed = JSON.parse(text);
    return { formatted: JSON.stringify(parsed, null, 2), isJson: true };
  } catch {
    return { formatted: text, isJson: false };
  }
}

function formatTimeFull(nanos: number): string {
  const ms = nanos / 1_000_000;
  return new Date(ms).toLocaleString();
}

export const WsMessageDetail = memo(({ message, onClose }: WsMessageDetailProps) => {
  const isSent = message.direction === "client_to_server";
  const { formatted, isJson } = useMemo(
    () => (message.is_binary ? { formatted: message.payload, isJson: false } : tryFormatJson(message.payload)),
    [message.payload, message.is_binary],
  );

  return (
    <div className="h-full flex flex-col bg-card">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border flex-shrink-0">
        <div className="flex items-center gap-2 text-sm font-medium">
          {isSent ? (
            <ArrowUp className="w-4 h-4 text-emerald-500" />
          ) : (
            <ArrowDown className="w-4 h-4 text-rose-500" />
          )}
          <span>{isSent ? "Sent" : "Received"}</span>
          <span className="text-muted-foreground">·</span>
          <span
            className={cn(
              "text-xs px-1.5 py-0.5 rounded",
              message.message_type === "text" && "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
              message.message_type === "binary" && "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300",
            )}
          >
            {message.message_type}
          </span>
          <span className="text-muted-foreground text-xs">{message.size} bytes</span>
        </div>
        <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onClose}>
          <X className="w-4 h-4" />
        </Button>
      </div>

      <div className="px-3 py-2 border-b border-border text-xs text-muted-foreground flex-shrink-0 space-y-1">
        <div>
          <span className="font-medium">Connection:</span> {message.connection_id}
        </div>
        <div>
          <span className="font-medium">Time:</span> {formatTimeFull(message.time)}
        </div>
        <div>
          <span className="font-medium">Sequence:</span> #{message.sequence}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-3">
        <pre
          className={cn(
            "text-xs font-mono whitespace-pre-wrap break-all",
            isJson && "text-foreground",
            message.is_binary && "text-muted-foreground",
          )}
        >
          {formatted}
        </pre>
      </div>
    </div>
  );
});
WsMessageDetail.displayName = "WsMessageDetail";
