import { Trans } from "@lingui/react/macro";

import { Badge } from "@/shared/ui";
import type { ErrorReport } from "../lib";

interface ErrorTabProps {
  report: ErrorReport;
}

export function ErrorTab({ report }: ErrorTabProps) {
  const maxCount = Math.max(...report.statusDistribution.map((s) => s.count), 1);

  return (
    <div className="space-y-6 mt-3">
      {/* Status Code Distribution */}
      <div>
        <h3 className="text-sm font-medium mb-3">
          <Trans>Error Status Code Distribution</Trans>
        </h3>
        {report.statusDistribution.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center">
            <Trans>No errors detected</Trans>
          </div>
        ) : (
          <div className="space-y-2">
            {report.statusDistribution.map((entry) => (
              <div key={entry.statusCode} className="flex items-center gap-3">
                <Badge
                  variant="outline"
                  className={`w-12 justify-center text-xs ${
                    entry.statusCode >= 500
                      ? "border-red-400 text-red-600 dark:text-red-400"
                      : "border-amber-400 text-amber-600 dark:text-amber-400"
                  }`}
                >
                  {entry.statusCode}
                </Badge>
                <div className="flex-1 h-5 bg-muted rounded overflow-hidden">
                  <div
                    className={`h-full rounded transition-all ${
                      entry.statusCode >= 500 ? "bg-red-500/70" : "bg-amber-500/70"
                    }`}
                    style={{ width: `${(entry.count / maxCount) * 100}%` }}
                  />
                </div>
                <span className="text-xs font-mono w-12 text-right">{entry.count}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Top Error Endpoints */}
      <div>
        <h3 className="text-sm font-medium mb-3">
          <Trans>Top Error Endpoints</Trans>
        </h3>
        {report.topErrorEndpoints.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center">
            <Trans>No error endpoints detected</Trans>
          </div>
        ) : (
          <div className="border rounded-lg overflow-hidden">
            <div className="grid grid-cols-[80px_1fr_80px_80px_100px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
              <div>
                <Trans>Method</Trans>
              </div>
              <div>
                <Trans>Path</Trans>
              </div>
              <div className="text-right">
                <Trans>Errors</Trans>
              </div>
              <div className="text-right">
                <Trans>Total</Trans>
              </div>
              <div className="text-right">
                <Trans>Error Rate</Trans>
              </div>
            </div>
            {report.topErrorEndpoints.map((ep, i) => (
              <div
                key={`${ep.method}-${ep.path}-${i}`}
                className="grid grid-cols-[80px_1fr_80px_80px_100px] gap-2 px-3 py-2 text-xs border-b last:border-b-0 hover:bg-muted/50"
              >
                <div>
                  <Badge variant="outline" className="text-[10px] px-1.5">
                    {ep.method}
                  </Badge>
                </div>
                <div className="truncate" title={ep.path}>
                  {ep.path}
                </div>
                <div className="text-right font-mono text-red-500">{ep.errorCount}</div>
                <div className="text-right font-mono">{ep.totalCount}</div>
                <div className="text-right font-mono">
                  <span className={ep.errorRate > 50 ? "text-red-500" : "text-amber-500"}>
                    {ep.errorRate.toFixed(1)}%
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
