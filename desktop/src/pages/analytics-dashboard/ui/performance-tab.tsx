import { useRef } from "react";
import { Trans } from "@lingui/react/macro";
import { useVirtualizer } from "@tanstack/react-virtual";

import { Badge } from "@/shared/ui";
import type { PerformanceReport } from "../lib";

interface PerformanceTabProps {
  report: PerformanceReport;
}

export function PerformanceTab({ report }: PerformanceTabProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: report.slowRequests.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 36,
    overscan: 10,
  });

  return (
    <div className="space-y-4 mt-3">
      {/* Percentile Summary */}
      <div className="flex gap-4">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">P50</span>
          <Badge variant="outline">{report.p50.toFixed(0)} ms</Badge>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">P95</span>
          <Badge variant="outline">{report.p95.toFixed(0)} ms</Badge>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">P99</span>
          <Badge variant="outline">{report.p99.toFixed(0)} ms</Badge>
        </div>
      </div>

      {/* Slow Requests Table */}
      <div className="border rounded-lg overflow-hidden">
        <div className="grid grid-cols-[80px_1fr_100px_80px_160px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
          <div>
            <Trans>Method</Trans>
          </div>
          <div>URL</div>
          <div className="text-right">
            <Trans>Duration</Trans>
          </div>
          <div className="text-right">
            <Trans>Status</Trans>
          </div>
          <div className="text-right">
            <Trans>Time</Trans>
          </div>
        </div>
        <div ref={parentRef} className="h-[400px] overflow-auto">
          {report.slowRequests.length === 0 ? (
            <div className="flex items-center justify-center h-20 text-sm text-muted-foreground">
              <Trans>No slow requests detected</Trans>
            </div>
          ) : (
            <div
              style={{
                height: `${rowVirtualizer.getTotalSize()}px`,
                position: "relative",
              }}
            >
              {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                const req = report.slowRequests[virtualRow.index];
                return (
                  <div
                    key={virtualRow.index}
                    className="grid grid-cols-[80px_1fr_100px_80px_160px] gap-2 px-3 items-center text-xs border-b hover:bg-muted/50"
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      height: `${virtualRow.size}px`,
                      transform: `translateY(${virtualRow.start}px)`,
                    }}
                  >
                    <div>
                      <Badge variant="outline" className="text-[10px] px-1.5">
                        {req.method}
                      </Badge>
                    </div>
                    <div className="truncate" title={req.url}>
                      {req.url}
                    </div>
                    <div className="text-right font-mono">
                      <span
                        className={
                          req.durationMs > 1000
                            ? "text-red-500"
                            : req.durationMs > 500
                              ? "text-amber-500"
                              : ""
                        }
                      >
                        {req.durationMs.toFixed(0)} ms
                      </span>
                    </div>
                    <div className="text-right">
                      {req.status != null && (
                        <span className={req.status >= 400 ? "text-red-500" : "text-green-600"}>
                          {req.status}
                        </span>
                      )}
                    </div>
                    <div className="text-right text-muted-foreground">
                      {new Date(req.timestamp).toLocaleTimeString()}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
