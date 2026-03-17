import { useMemo, useCallback, useRef, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { Trash2, Box } from "lucide-react";

import { useGrpcStore, useTransactionStore } from "@/shared/stores";
import { extractGrpcCall } from "@/shared/stores/grpc-store";
import { GrpcCallTable, GrpcCallDetail } from "@/widgets/grpc-calls";
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
import type { GrpcCall } from "@/entities/grpc";

export const GrpcDashboard = () => {
  const transactions = useTransactionStore((s) => s.transactions);
  const selectedCall = useGrpcStore((s) => s.selectedCall);
  const setSelectedCall = useGrpcStore((s) => s.setSelectedCall);

  const calls = useMemo(() => {
    const result: GrpcCall[] = [];
    for (const tx of transactions) {
      const call = extractGrpcCall(tx);
      if (call) result.push(call);
    }
    return result;
  }, [transactions]);

  const serviceCounts = useMemo(() => {
    const counts = new Map<string, number>();
    for (const call of calls) {
      const key = call.service ?? "(unknown)";
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return counts;
  }, [calls]);

  const errorCount = useMemo(
    () => calls.filter((c) => c.statusCode != null && c.statusCode !== 0).length,
    [calls],
  );

  const handleSelectCall = useCallback(
    (call: GrpcCall) => {
      setSelectedCall(selectedCall?.id === call.id ? null : call);
    },
    [selectedCall, setSelectedCall],
  );

  const handleCloseDetail = useCallback(() => {
    setSelectedCall(null);
  }, [setSelectedCall]);

  const handleClear = useCallback(() => {
    setSelectedCall(null);
  }, [setSelectedCall]);

  const detailPanelRef = useRef<PanelImperativeHandle>(null);

  useEffect(() => {
    if (selectedCall) {
      detailPanelRef.current?.expand();
    } else {
      detailPanelRef.current?.collapse();
    }
  }, [selectedCall]);

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "grpc-dashboard-layout",
    storage: localStorage,
  });

  return (
    <div className="flex-1 flex flex-col h-full overflow-x-hidden">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-border bg-background flex-shrink-0">
        <div className="flex items-center gap-2">
          <Box className="w-4 h-4 text-muted-foreground" />
          <h1 className="text-sm font-semibold">
            <Trans>gRPC</Trans>
          </h1>
          <span className="text-xs text-muted-foreground">
            {calls.length} <Trans>calls</Trans>
            {serviceCounts.size > 0 && (
              <>
                {" "}
                · {serviceCounts.size} <Trans>services</Trans>
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
              "grpc-calls": 100,
            }
          }
          onLayoutChanged={onLayoutChanged}
          className="flex-1 flex bg-background"
        >
          {/* Call table */}
          <ResizablePanel id="grpc-calls" className="flex flex-1 h-full overflow-hidden">
            <GrpcCallTable
              calls={calls}
              selectedCall={selectedCall}
              onSelectCall={handleSelectCall}
            />
          </ResizablePanel>

          <ResizableHandle withHandle />
          <ResizablePanel
            panelRef={detailPanelRef}
            id="grpc-detail"
            maxSize="50%"
            minSize="20%"
            defaultSize="0%"
            collapsible
            className="h-full overflow-hidden"
          >
            {selectedCall && (
              <GrpcCallDetail call={selectedCall} onClose={handleCloseDetail} />
            )}
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    </div>
  );
};
