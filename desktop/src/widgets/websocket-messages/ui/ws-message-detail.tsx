import { memo, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { ArrowUp, ArrowDown, X, Play } from "lucide-react";
import { Editor } from "@monaco-editor/react";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { useMonacoTheme } from "@/shared/hooks/use-monaco-theme";
import { cn } from "@/shared/lib";
import { formatBytes } from "@/shared/lib/format-bytes";
import { getWsContentView, parseMqtt } from "@/shared/lib/ws-content-view";
import { formatTimeFull } from "@/shared/lib/format-time";
import type { WsMessageInfo } from "@/entities/websocket";

interface WsMessageDetailProps {
  message: WsMessageInfo;
  onClose: () => void;
  onReplayRequest: (message: WsMessageInfo) => void;
}

export const WsMessageDetail = memo(
  ({ message, onClose, onReplayRequest }: WsMessageDetailProps) => {
    const { t } = useLingui();
    const { theme, beforeMount } = useMonacoTheme();
    const isSent = message.direction === "client_to_server";

    const mqttParsed = useMemo(
      () =>
        message.content_type === "mqtt" ? parseMqtt(message.payload, message.mqtt_version) : null,
      [message.payload, message.content_type, message.mqtt_version],
    );

    const { language, formatted } = useMemo(
      () =>
        getWsContentView(
          message.payload,
          message.is_binary,
          message.content_type,
          message.mqtt_version,
        ),
      [message.payload, message.is_binary, message.content_type, message.mqtt_version],
    );

    const metaItems = useMemo(() => {
      const items = [
        {
          label: t`Direction`,
          value: isSent ? t`Sent (Client → Server)` : t`Received (Server → Client)`,
        },
        {
          label: t`Type`,
          value: mqttParsed ? `MQTT ${mqttParsed.meta.packetType}` : message.message_type,
        },
        { label: t`Size`, value: formatBytes(message.size) },
        { label: t`Time`, value: formatTimeFull(message.time) },
        { label: t`Connection`, value: message.connection_id },
        { label: t`Sequence`, value: `#${message.sequence}` },
      ];

      if (mqttParsed) {
        for (const field of mqttParsed.meta.fields) {
          items.push({ label: field.label, value: field.value });
        }
      }

      return items;
    }, [message, isSent, mqttParsed]);

    return (
      <div className="h-full flex flex-col bg-card select-text">
        {/* Header bar */}
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-border flex-shrink-0 bg-muted/30">
          <div className="flex items-center gap-2 text-xs font-medium">
            {isSent ? (
              <ArrowUp className="w-3.5 h-3.5 text-emerald-500" />
            ) : (
              <ArrowDown className="w-3.5 h-3.5 text-rose-500" />
            )}
            <span>{isSent ? t`Sent` : t`Received`}</span>
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
            {message.content_type && message.content_type !== "plain" && (
              <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-300">
                {message.content_type === "socket_io" ? "Socket.IO" : "MQTT"}
              </span>
            )}
            <span className="text-muted-foreground">{formatBytes(message.size)}</span>
          </div>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-xs gap-1"
              title={t`Replay message`}
              onClick={() => onReplayRequest(message)}
            >
              <Play className="w-3 h-3" />
              <Trans>Replay</Trans>
            </Button>
            <Tooltip>
              <TooltipTrigger render={<div />}>
                <Button variant="ghost" size="sm" className="h-6 w-6 p-0" onClick={onClose}>
                  <X className="w-3.5 h-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent sideOffset={4}>
                <Trans>Close</Trans>
              </TooltipContent>
            </Tooltip>
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
            beforeMount={beforeMount}
            theme={theme}
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
      </div>
    );
  },
);
WsMessageDetail.displayName = "WsMessageDetail";
