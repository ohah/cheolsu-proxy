import { memo, useMemo } from "react";
import { useLingui } from "@lingui/react/macro";
import { Trans } from "@lingui/react/macro";
import { X } from "lucide-react";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { cn } from "@/shared/lib";
import { formatBytes } from "@/shared/lib/format-bytes";
import type { GrpcCall } from "@/entities/grpc";
import { GRPC_STATUS_COLORS } from "./grpc-status-colors";

interface GrpcCallDetailProps {
  call: GrpcCall;
  onClose: () => void;
}

function formatTimeFull(nanos: number): string {
  const ms = nanos / 1_000_000;
  return new Date(ms).toLocaleString();
}

function formatDuration(requestNanos: number, responseNanos: number | null): string {
  if (responseNanos == null) return "-";
  const ms = (responseNanos - requestNanos) / 1_000_000;
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export const GrpcCallDetail = memo(({ call, onClose }: GrpcCallDetailProps) => {
  const { t } = useLingui();
  const isError = call.statusCode != null && call.statusCode !== 0;

  const metaItems = useMemo(() => {
    const items: { label: string; value: string; highlight?: boolean }[] = [];

    if (call.service) {
      items.push({ label: t`Service`, value: call.service });
    }
    if (call.method) {
      items.push({ label: t`Method`, value: call.method });
    }
    items.push({ label: t`Endpoint`, value: call.endpoint });
    items.push({ label: t`Time`, value: formatTimeFull(call.requestTime) });
    items.push({
      label: t`Duration`,
      value: formatDuration(call.requestTime, call.responseTime),
    });

    if (call.statusName != null) {
      items.push({
        label: t`gRPC Status`,
        value: `${call.statusCode} ${call.statusName}`,
        highlight: isError,
      });
    }
    if (call.statusMessage) {
      items.push({ label: t`Status Message`, value: call.statusMessage, highlight: isError });
    }
    if (call.httpStatus != null) {
      items.push({ label: t`HTTP Status`, value: String(call.httpStatus) });
    }
    if (call.contentSubtype) {
      items.push({ label: t`Encoding`, value: call.contentSubtype });
    }
    items.push({
      label: t`Request`,
      value: `${call.requestFrameCount} frame(s), ${formatBytes(call.requestSize)}`,
    });
    items.push({
      label: t`Response`,
      value: `${call.responseFrameCount} frame(s), ${formatBytes(call.responseSize)}`,
    });

    return items;
  }, [call, t, isError]);

  return (
    <div className="h-full flex flex-col bg-card select-text">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border flex-shrink-0 bg-muted/30">
        <div className="flex items-center gap-2 text-xs font-medium">
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300">
            gRPC
          </span>
          <span className="font-mono text-foreground">
            {call.service ? `${call.service}/` : ""}
            {call.method ?? ""}
          </span>
          {call.statusName && (
            <span
              className={cn(
                "text-[10px] font-medium",
                GRPC_STATUS_COLORS[call.statusName] ?? (isError ? "text-destructive" : "text-muted-foreground"),
              )}
            >
              {call.statusName}
            </span>
          )}
        </div>
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

      {/* Meta info */}
      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <tbody>
            {metaItems.map((item) => (
              <tr key={item.label} className="border-b border-border/30 last:border-b-0">
                <td className="px-3 py-1.5 text-muted-foreground font-medium whitespace-nowrap w-28 align-top">
                  {item.label}
                </td>
                <td
                  className={cn(
                    "px-3 py-1.5 font-mono text-foreground break-all",
                    item.highlight && "text-destructive",
                  )}
                >
                  {item.value}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
});
GrpcCallDetail.displayName = "GrpcCallDetail";
