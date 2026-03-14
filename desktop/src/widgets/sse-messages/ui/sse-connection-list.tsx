import { memo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Plug, PlugZap } from "lucide-react";
import { cn } from "@/shared/lib";
import type { SseConnection } from "@/entities/sse";

interface SseConnectionListProps {
  connections: SseConnection[];
  selectedConnectionId: string | null;
  onSelectConnection: (id: string | null) => void;
  eventCounts: Map<string, number>;
}

export const SseConnectionList = memo(
  ({
    connections,
    selectedConnectionId,
    onSelectConnection,
    eventCounts,
  }: SseConnectionListProps) => {
    const { t } = useLingui();

    if (connections.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm p-4">
          <Trans>No active SSE connections</Trans>
        </div>
      );
    }

    return (
      <div className="flex-1 overflow-y-auto">
        <div
          className={cn(
            "px-3 py-2 cursor-pointer text-sm border-b border-border transition-colors",
            selectedConnectionId === null
              ? "bg-accent text-accent-foreground"
              : "hover:bg-muted/50",
          )}
          onClick={() => onSelectConnection(null)}
        >
          <span className="font-medium">
            <Trans>All Connections</Trans>
          </span>
          <span className="ml-2 text-xs text-muted-foreground">({connections.length})</span>
        </div>
        {connections.map((conn) => {
          const isSelected = selectedConnectionId === conn.id;
          const isConnected = conn.status === "connected";
          const count = eventCounts.get(conn.id) ?? 0;

          let displayUri = conn.uri;
          try {
            const url = new URL(conn.uri);
            displayUri = url.host + url.pathname;
          } catch {
            // URI 파싱 실패 시 원본 사용
          }

          return (
            <div
              key={conn.id}
              className={cn(
                "px-3 py-2 cursor-pointer border-b border-border/50 transition-colors",
                isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
              )}
              onClick={() => onSelectConnection(conn.id)}
            >
              <div className="flex items-center gap-2">
                {isConnected ? (
                  <PlugZap className="w-3 h-3 text-emerald-500 flex-shrink-0" />
                ) : (
                  <Plug className="w-3 h-3 text-muted-foreground flex-shrink-0" />
                )}
                <span className="text-xs truncate flex-1" title={conn.uri}>
                  {displayUri}
                </span>
                <span className="text-[10px] text-muted-foreground flex-shrink-0">
                  {count} {t`events`}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    );
  },
);
SseConnectionList.displayName = "SseConnectionList";
