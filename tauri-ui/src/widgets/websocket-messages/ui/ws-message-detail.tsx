import { memo, useMemo, useState } from "react";
import { ArrowUp, ArrowDown, X, Play } from "lucide-react";
import { Editor } from "@monaco-editor/react";
import { Button } from "@/shared/ui";
import { cn } from "@/shared/lib";
import type { WsMessageInfo } from "@/entities/websocket";
import { WsReplayDialog } from "@/features/websocket-replay";

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

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function detectLanguage(payload: string, isBinary: boolean, isJson: boolean): string {
  if (isBinary) return "plaintext";
  if (isJson) return "json";
  const trimmed = payload.trimStart();
  if (trimmed.startsWith("<")) return "xml";
  return "plaintext";
}

export const WsMessageDetail = memo(({ message, onClose }: WsMessageDetailProps) => {
  const [replayOpen, setReplayOpen] = useState(false);
  const isSent = message.direction === "client_to_server";
  const { formatted, isJson } = useMemo(
    () =>
      message.is_binary
        ? { formatted: message.payload, isJson: false }
        : tryFormatJson(message.payload),
    [message.payload, message.is_binary],
  );

  const language = useMemo(
    () => detectLanguage(message.payload, message.is_binary, isJson),
    [message.payload, message.is_binary, isJson],
  );

  const metaItems = [
    { label: "Direction", value: isSent ? "Sent (Client → Server)" : "Received (Server → Client)" },
    { label: "Type", value: message.message_type },
    { label: "Size", value: formatSize(message.size) },
    { label: "Time", value: formatTimeFull(message.time) },
    { label: "Connection", value: message.connection_id },
    { label: "Sequence", value: `#${message.sequence}` },
  ];

  return (
    <div className="h-full flex flex-col bg-card">
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border flex-shrink-0 bg-muted/30">
        <div className="flex items-center gap-2 text-xs font-medium">
          {isSent ? (
            <ArrowUp className="w-3.5 h-3.5 text-emerald-500" />
          ) : (
            <ArrowDown className="w-3.5 h-3.5 text-rose-500" />
          )}
          <span>{isSent ? "Sent" : "Received"}</span>
          <span className="text-muted-foreground">·</span>
          <span
            className={cn(
              "px-1.5 py-0.5 rounded text-[10px] font-medium",
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
            {message.message_type}
          </span>
          <span className="text-muted-foreground">{formatSize(message.size)}</span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs gap-1"
            title="Replay message"
            onClick={() => setReplayOpen(true)}
          >
            <Play className="w-3 h-3" />
            Replay
          </Button>
          <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onClose}>
            <X className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      {/* Meta info - DevTools style key-value grid */}
      <div className="border-b border-border flex-shrink-0">
        <table className="w-full text-xs">
          <tbody>
            {metaItems.map((item) => (
              <tr key={item.label} className="border-b border-border/30 last:border-b-0">
                <td className="px-3 py-1 text-muted-foreground font-medium whitespace-nowrap w-24 align-top">
                  {item.label}
                </td>
                <td className="px-3 py-1 font-mono text-foreground break-all">{item.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Payload - Monaco Editor */}
      <div className="flex-1 overflow-hidden">
        <Editor
          height="100%"
          language={language}
          value={formatted}
          options={{
            readOnly: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 12,
            lineNumbers: "off",
            wordWrap: "on",
            automaticLayout: true,
            padding: { top: 8, bottom: 8 },
            scrollbar: {
              vertical: "auto",
              horizontal: "auto",
            },
            renderLineHighlight: "none",
            overviewRulerLanes: 0,
            hideCursorInOverviewRuler: true,
            overviewRulerBorder: false,
            contextmenu: false,
          }}
        />
      </div>

      <WsReplayDialog message={message} open={replayOpen} onOpenChange={setReplayOpen} />
    </div>
  );
});
WsMessageDetail.displayName = "WsMessageDetail";
