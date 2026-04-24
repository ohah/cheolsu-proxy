import { memo, useMemo } from "react";
import { useLingui } from "@lingui/react/macro";
import { Trans } from "@lingui/react/macro";
import { X } from "lucide-react";
import { Editor } from "@monaco-editor/react";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { useMonacoTheme } from "@/shared/hooks/use-monaco-theme";
import { formatBytes } from "@/shared/lib/format-bytes";
import { formatTimeFull } from "@/shared/lib/format-time";
import type { SseEventInfo } from "@/entities/sse";

interface SseMessageDetailProps {
  event: SseEventInfo;
  onClose: () => void;
}

function detectLanguage(data: string): string {
  const trimmed = data.trim();
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    return "json";
  }
  if (trimmed.startsWith("<") && trimmed.endsWith(">")) {
    return "xml";
  }
  return "plaintext";
}

function tryFormatJson(data: string): string {
  try {
    const parsed = JSON.parse(data);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return data;
  }
}

export const SseMessageDetail = memo(({ event, onClose }: SseMessageDetailProps) => {
  const { t } = useLingui();
  const { theme, beforeMount } = useMonacoTheme();

  const language = useMemo(() => detectLanguage(event.data), [event.data]);
  const formatted = useMemo(
    () => (language === "json" ? tryFormatJson(event.data) : event.data),
    [event.data, language],
  );

  const metaItems = useMemo(() => {
    const items = [
      { label: t`Event Type`, value: event.event_type ?? "message" },
      { label: t`Size`, value: formatBytes(event.size) },
      { label: t`Time`, value: formatTimeFull(event.time) },
      { label: t`Connection`, value: event.connection_id },
      { label: t`Sequence`, value: `#${event.sequence}` },
    ];

    if (event.id != null) {
      items.push({ label: t`Event ID`, value: event.id });
    }
    if (event.retry != null) {
      items.push({ label: t`Retry`, value: `${event.retry} ms` });
    }

    return items;
  }, [event]);

  return (
    <div className="h-full flex flex-col bg-card select-text">
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border flex-shrink-0 bg-muted/30">
        <div className="flex items-center gap-2 text-xs font-medium">
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
            {event.event_type ?? "message"}
          </span>
          <span className="text-muted-foreground">{formatBytes(event.size)}</span>
        </div>
        <div className="flex items-center gap-1">
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

      {/* Meta info */}
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
});
SseMessageDetail.displayName = "SseMessageDetail";
