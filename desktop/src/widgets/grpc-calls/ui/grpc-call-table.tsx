import { memo, useCallback, useRef, useEffect, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "@/shared/lib";
import { ScrollArea } from "@/shared/ui";
import type { GrpcCall } from "@/entities/grpc";
import { GRPC_STATUS_COLORS } from "./grpc-status-colors";

interface GrpcCallTableProps {
  calls: GrpcCall[];
  selectedCall: GrpcCall | null;
  onSelectCall: (call: GrpcCall) => void;
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

function formatDuration(requestNanos: number, responseNanos: number | null): string {
  if (responseNanos == null) return "-";
  const ms = (responseNanos - requestNanos) / 1_000_000;
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

const GrpcCallRow = memo(
  ({
    call,
    isSelected,
    onSelect,
  }: {
    call: GrpcCall;
    isSelected: boolean;
    onSelect: () => void;
  }) => {
    const isError = call.statusCode != null && call.statusCode !== 0;

    return (
      <div
        className={cn(
          "h-8 flex items-center cursor-pointer border-b border-border/50 text-xs transition-colors",
          isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
        )}
        onClick={onSelect}
      >
        <div className="px-2 w-20 flex-shrink-0">
          {call.statusName != null ? (
            <span
              className={cn(
                "text-[10px] font-medium",
                GRPC_STATUS_COLORS[call.statusName] ?? "text-muted-foreground",
              )}
            >
              {call.statusName}
            </span>
          ) : (
            <span className="text-[10px] text-muted-foreground">-</span>
          )}
        </div>
        <div className="px-2 flex-1 truncate min-w-0">
          <span className={cn("font-mono text-muted-foreground text-[10px]")}>
            {call.service ? `${call.service}/` : ""}
          </span>
          <span className={cn("font-mono text-foreground", isError && "text-destructive")}>
            {call.method ?? call.endpoint}
          </span>
        </div>
        <div className="px-2 w-16 text-right text-muted-foreground flex-shrink-0">
          {formatDuration(call.requestTime, call.responseTime)}
        </div>
        <div className="px-2 pr-4 w-24 text-right text-muted-foreground flex-shrink-0">
          {formatTime(call.requestTime)}
        </div>
      </div>
    );
  },
);
GrpcCallRow.displayName = "GrpcCallRow";

const ROW_HEIGHT = 32;

export const GrpcCallTable = memo(
  ({ calls, selectedCall, onSelectCall }: GrpcCallTableProps) => {
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
      count: calls.length,
      getScrollElement,
      estimateSize,
      overscan: 20,
    });

    useEffect(() => {
      if (shouldAutoScroll.current && calls.length > 0) {
        virtualizer.scrollToIndex(calls.length - 1, { align: "end" });
      }
    }, [calls.length]);

    const selectHandlers = useMemo(
      () => calls.map((call) => () => onSelectCall(call)),
      [calls, onSelectCall],
    );

    if (calls.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          <Trans>gRPC 호출을 기다리는 중...</Trans>
        </div>
      );
    }

    return (
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        <div className="flex-shrink-0 border-b border-border">
          <div className="h-7 flex items-center text-xs text-muted-foreground font-medium bg-muted/30">
            <div className="px-2 w-20 text-left flex-shrink-0">{t`Status`}</div>
            <div className="px-2 flex-1 text-left min-w-0">{t`Method`}</div>
            <div className="px-2 w-16 text-right flex-shrink-0">{t`Duration`}</div>
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
              const call = calls[virtualItem.index];
              return (
                <div
                  key={call.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <GrpcCallRow
                    call={call}
                    isSelected={selectedCall?.id === call.id}
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
GrpcCallTable.displayName = "GrpcCallTable";
