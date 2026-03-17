import { memo, useCallback, useRef, useEffect, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "@/shared/lib";
import { ScrollArea } from "@/shared/ui";
import { formatTime, formatDuration } from "@/shared/lib/format-time";
import type { GraphqlOperation } from "@/entities/graphql";

interface GraphqlOperationTableProps {
  operations: GraphqlOperation[];
  selectedOperation: GraphqlOperation | null;
  onSelectOperation: (op: GraphqlOperation) => void;
}

const TYPE_COLORS: Record<string, string> = {
  query: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
  mutation: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300",
  subscription: "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300",
};

const GraphqlOperationRow = memo(
  ({
    operation,
    isSelected,
    onSelect,
  }: {
    operation: GraphqlOperation;
    isSelected: boolean;
    onSelect: () => void;
  }) => {
    const hasErrors = operation.responseErrors && operation.responseErrors.length > 0;

    return (
      <div
        className={cn(
          "h-8 flex items-center cursor-pointer border-b border-border/50 text-xs transition-colors",
          isSelected ? "bg-accent text-accent-foreground" : "hover:bg-muted/50",
        )}
        onClick={onSelect}
      >
        <div className="px-2 w-24 flex-shrink-0">
          <span
            className={cn(
              "inline-block px-1.5 py-0.5 rounded text-[10px] font-medium uppercase",
              TYPE_COLORS[operation.operationType] ?? TYPE_COLORS.query,
            )}
          >
            {operation.operationType === "subscription"
              ? "SUB"
              : operation.operationType.slice(0, 1).toUpperCase()}
          </span>
          {operation.batchSize != null && (
            <span className="ml-1 text-[10px] text-muted-foreground">
              [{(operation.batchIndex ?? 0) + 1}/{operation.batchSize}]
            </span>
          )}
        </div>
        <div className="px-2 flex-1 truncate min-w-0">
          <span className={cn("font-mono text-foreground", hasErrors && "text-destructive")}>
            {operation.operationName ?? "(anonymous)"}
          </span>
        </div>
        <div className="px-2 w-16 text-center flex-shrink-0">
          {operation.statusCode != null && (
            <span
              className={cn(
                "text-[10px] font-mono",
                operation.statusCode >= 400 ? "text-destructive" : "text-muted-foreground",
              )}
            >
              {operation.statusCode}
            </span>
          )}
        </div>
        <div className="px-2 w-16 text-right text-muted-foreground flex-shrink-0">
          {formatDuration(operation.requestTime, operation.responseTime)}
        </div>
        <div className="px-2 pr-4 w-24 text-right text-muted-foreground flex-shrink-0">
          {formatTime(operation.requestTime)}
        </div>
      </div>
    );
  },
);
GraphqlOperationRow.displayName = "GraphqlOperationRow";

const ROW_HEIGHT = 32;

export const GraphqlOperationTable = memo(
  ({ operations, selectedOperation, onSelectOperation }: GraphqlOperationTableProps) => {
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
      count: operations.length,
      getScrollElement,
      estimateSize,
      overscan: 20,
    });

    useEffect(() => {
      if (shouldAutoScroll.current && operations.length > 0) {
        virtualizer.scrollToIndex(operations.length - 1, { align: "end" });
      }
    }, [operations.length]);

    const selectHandlers = useMemo(
      () => operations.map((op) => () => onSelectOperation(op)),
      [operations, onSelectOperation],
    );

    if (operations.length === 0) {
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          <Trans>GraphQL 오퍼레이션을 기다리는 중...</Trans>
        </div>
      );
    }

    return (
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        <div className="flex-shrink-0 border-b border-border">
          <div className="h-7 flex items-center text-xs text-muted-foreground font-medium bg-muted/30">
            <div className="px-2 w-24 text-left flex-shrink-0">{t`Type`}</div>
            <div className="px-2 flex-1 text-left min-w-0">{t`Operation`}</div>
            <div className="px-2 w-16 text-center flex-shrink-0">{t`Status`}</div>
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
              const op = operations[virtualItem.index];
              return (
                <div
                  key={op.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                  }}
                >
                  <GraphqlOperationRow
                    operation={op}
                    isSelected={selectedOperation?.id === op.id}
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
GraphqlOperationTable.displayName = "GraphqlOperationTable";
