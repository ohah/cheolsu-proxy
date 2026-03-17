import { useRef } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { ArrowUpDown } from "lucide-react";
import { useVirtualizer } from "@tanstack/react-virtual";

import { Badge, Button } from "@/shared/ui";
import type { EndpointStat, EndpointSortKey } from "../lib";

interface EndpointTabProps {
  endpoints: EndpointStat[];
  sortBy: EndpointSortKey;
  onSortChange: (sort: EndpointSortKey) => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const k = 1024;
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / k ** i).toFixed(1)} ${units[i]}`;
}

export function EndpointTab({ endpoints, sortBy, onSortChange }: EndpointTabProps) {
  const { t } = useLingui();
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: endpoints.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 36,
    overscan: 10,
  });

  const sortOptions: { key: EndpointSortKey; label: string }[] = [
    { key: "count", label: t`Frequency` },
    { key: "avgDuration", label: t`Response Time` },
    { key: "errorRate", label: t`Error Rate` },
  ];

  return (
    <div className="space-y-3 mt-3">
      {/* Sort Controls */}
      <div className="flex items-center gap-2">
        <ArrowUpDown className="w-3.5 h-3.5 text-muted-foreground" />
        <span className="text-xs text-muted-foreground">
          <Trans>Sort by</Trans>:
        </span>
        {sortOptions.map((opt) => (
          <Button
            key={opt.key}
            variant={sortBy === opt.key ? "secondary" : "ghost"}
            size="sm"
            className="h-6 text-xs px-2"
            onClick={() => onSortChange(opt.key)}
          >
            {opt.label}
          </Button>
        ))}
      </div>

      {/* Table */}
      <div className="border rounded-lg overflow-hidden">
        <div className="grid grid-cols-[80px_1fr_60px_90px_90px_80px_90px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
          <div>
            <Trans>Method</Trans>
          </div>
          <div>
            <Trans>Path</Trans>
          </div>
          <div className="text-right">
            <Trans>Count</Trans>
          </div>
          <div className="text-right">
            <Trans>Avg Time</Trans>
          </div>
          <div className="text-right">P95</div>
          <div className="text-right">
            <Trans>Error Rate</Trans>
          </div>
          <div className="text-right">
            <Trans>Avg Size</Trans>
          </div>
        </div>
        <div ref={parentRef} className="h-[400px] overflow-auto">
          {endpoints.length === 0 ? (
            <div className="flex items-center justify-center h-20 text-sm text-muted-foreground">
              <Trans>No endpoint data</Trans>
            </div>
          ) : (
            <div
              style={{
                height: `${rowVirtualizer.getTotalSize()}px`,
                position: "relative",
              }}
            >
              {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                const ep = endpoints[virtualRow.index];
                return (
                  <div
                    key={virtualRow.index}
                    className="grid grid-cols-[80px_1fr_60px_90px_90px_80px_90px] gap-2 px-3 items-center text-xs border-b hover:bg-muted/50"
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
                        {ep.method}
                      </Badge>
                    </div>
                    <div className="truncate" title={ep.path}>
                      {ep.path}
                    </div>
                    <div className="text-right font-mono">{ep.count}</div>
                    <div className="text-right font-mono">{ep.avgDuration.toFixed(0)} ms</div>
                    <div className="text-right font-mono">{ep.p95Duration.toFixed(0)} ms</div>
                    <div className="text-right font-mono">
                      <span
                        className={
                          ep.errorRate > 10
                            ? "text-red-500"
                            : ep.errorRate > 0
                              ? "text-amber-500"
                              : ""
                        }
                      >
                        {ep.errorRate.toFixed(1)}%
                      </span>
                    </div>
                    <div className="text-right font-mono">{formatBytes(ep.avgResponseSize)}</div>
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
