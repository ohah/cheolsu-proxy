import { useMemo, useCallback, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { Trash2, Braces } from "lucide-react";

import { useGraphqlStore, useTransactionStore } from "@/shared/stores";
import { extractGraphqlOperations } from "@/shared/stores/graphql-store";
import {
  GraphqlOperationTable,
  GraphqlOperationDetail,
} from "@/widgets/graphql-operations";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
  Button,
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/shared/ui";
import { useDefaultLayout, type PanelImperativeHandle } from "react-resizable-panels";
import type { GraphqlOperation } from "@/entities/graphql";

export const GraphqlDashboard = () => {
  const transactions = useTransactionStore((s) => s.transactions);
  const selectedOperation = useGraphqlStore((s) => s.selectedOperation);
  const setSelectedOperation = useGraphqlStore((s) => s.setSelectedOperation);

  const operations = useMemo(() => {
    const ops: GraphqlOperation[] = [];
    for (const tx of transactions) {
      const extracted = extractGraphqlOperations(tx);
      ops.push(...extracted);
    }
    return ops;
  }, [transactions]);

  const endpointCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const op of operations) {
      try {
        const url = new URL(op.endpoint);
        const key = url.host + url.pathname;
        counts.set(key, (counts.get(key) ?? 0) + 1);
      } catch {
        counts.set(op.endpoint, (counts.get(op.endpoint) ?? 0) + 1);
      }
    }
    return counts;
  }, [operations]);

  const errorCount = useMemo(
    () =>
      operations.filter(
        (op) => op.responseErrors && op.responseErrors.length > 0,
      ).length,
    [operations],
  );

  const handleSelectOperation = useCallback(
    (operation: GraphqlOperation) => {
      setSelectedOperation(
        selectedOperation?.id === operation.id ? null : operation,
      );
    },
    [selectedOperation, setSelectedOperation],
  );

  const handleCloseDetail = useCallback(() => {
    setSelectedOperation(null);
  }, [setSelectedOperation]);

  const handleClear = useCallback(() => {
    setSelectedOperation(null);
  }, [setSelectedOperation]);

  const detailPanelRef = useRef<PanelImperativeHandle>(null);

  useEffect(() => {
    if (selectedOperation) {
      detailPanelRef.current?.expand();
    } else {
      detailPanelRef.current?.collapse();
    }
  }, [selectedOperation]);

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "graphql-dashboard-layout",
    storage: localStorage,
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-x-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-background flex-shrink-0">
        <div className="flex items-center gap-2">
          <Braces className="w-4 h-4 text-muted-foreground" />
          <h1 className="text-sm font-semibold">
            <Trans>GraphQL</Trans>
          </h1>
          <span className="text-xs text-muted-foreground">
            {operations.length} <Trans>operations</Trans>
            {endpointCounts.size > 0 && (
              <>
                {" "}
                · {endpointCounts.size} <Trans>endpoints</Trans>
              </>
            )}
            {errorCount > 0 && (
              <span className="text-destructive ml-1">
                · {errorCount} <Trans>errors</Trans>
              </span>
            )}
          </span>
        </div>
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Button variant="ghost" size="sm" onClick={handleClear}>
              <Trash2 className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent sideOffset={4}>
            <Trans>Clear selection</Trans>
          </TooltipContent>
        </Tooltip>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        <ResizablePanelGroup
          orientation="horizontal"
          defaultLayout={
            defaultLayout ?? {
              "graphql-operations": 100,
            }
          }
          onLayoutChanged={onLayoutChanged}
          className="flex-1 flex bg-background"
        >
          {/* Operation table */}
          <ResizablePanel
            id="graphql-operations"
            className="flex flex-1 h-full overflow-hidden"
          >
            <GraphqlOperationTable
              operations={operations}
              selectedOperation={selectedOperation}
              onSelectOperation={handleSelectOperation}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />
          <ResizablePanel
            panelRef={detailPanelRef}
            id="graphql-detail"
            maxSize="50%"
            minSize="20%"
            defaultSize="0%"
            collapsible
            className="h-full overflow-hidden"
          >
            {selectedOperation && (
              <GraphqlOperationDetail
                operation={selectedOperation}
                onClose={handleCloseDetail}
              />
            )}
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};
