import { Trans } from "@lingui/react/macro";

import { Badge } from "@/shared/ui";
import { formatBytes } from "@/shared/lib/format-bytes";
import { formatTime } from "@/shared/lib/format-time";
import type { MiscReport } from "../lib";

interface MiscTabProps {
  report: MiscReport;
}

export function MiscTab({ report }: MiscTabProps) {
  return (
    <div className="space-y-6 mt-3">
      {/* CORS Issues */}
      <div>
        <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
          <Trans>CORS Issues</Trans>
          {report.corsIssues.length > 0 && (
            <Badge variant="destructive" className="text-[10px]">
              {report.corsIssues.length}
            </Badge>
          )}
        </h3>
        {report.corsIssues.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center">
            <Trans>No CORS issues detected</Trans>
          </div>
        ) : (
          <div className="border rounded-lg overflow-hidden max-h-[200px] overflow-y-auto">
            <div className="grid grid-cols-[80px_1fr_1fr_140px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
              <div>
                <Trans>Method</Trans>
              </div>
              <div>URL</div>
              <div>
                <Trans>Issue</Trans>
              </div>
              <div className="text-right">
                <Trans>Time</Trans>
              </div>
            </div>
            {report.corsIssues.map((issue, i) => (
              <div
                key={`cors-${i}`}
                className="grid grid-cols-[80px_1fr_1fr_140px] gap-2 px-3 py-2 text-xs border-b last:border-b-0 hover:bg-muted/50"
              >
                <div>
                  <Badge variant="outline" className="text-[10px] px-1.5">
                    {issue.method}
                  </Badge>
                </div>
                <div className="truncate" title={issue.url}>
                  {issue.url}
                </div>
                <div className="text-red-500 truncate" title={issue.issue}>
                  {issue.issue}
                </div>
                <div className="text-right text-muted-foreground">
                  {formatTime(issue.timestamp)}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Mixed Content */}
      <div>
        <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
          <Trans>Mixed Content Warnings</Trans>
          {report.mixedContentWarnings.length > 0 && (
            <Badge variant="secondary" className="text-[10px]">
              {report.mixedContentWarnings.length}
            </Badge>
          )}
        </h3>
        {report.mixedContentWarnings.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center">
            <Trans>No mixed content warnings</Trans>
          </div>
        ) : (
          <div className="border rounded-lg overflow-hidden max-h-[200px] overflow-y-auto">
            <div className="grid grid-cols-[1fr_1fr_140px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
              <div>URL (HTTP)</div>
              <div>
                <Trans>Referrer</Trans> (HTTPS)
              </div>
              <div className="text-right">
                <Trans>Time</Trans>
              </div>
            </div>
            {report.mixedContentWarnings.map((warning, i) => (
              <div
                key={`mixed-${i}`}
                className="grid grid-cols-[1fr_1fr_140px] gap-2 px-3 py-2 text-xs border-b last:border-b-0 hover:bg-muted/50"
              >
                <div className="truncate text-amber-600" title={warning.url}>
                  {warning.url}
                </div>
                <div className="truncate text-muted-foreground" title={warning.referrer}>
                  {warning.referrer}
                </div>
                <div className="text-right text-muted-foreground">
                  {formatTime(warning.timestamp)}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Payload Size Analysis */}
      <div>
        <h3 className="text-sm font-medium mb-3">
          <Trans>Largest Payloads (Top 10)</Trans>
        </h3>
        {report.largestPayloads.length === 0 ? (
          <div className="text-sm text-muted-foreground py-4 text-center">
            <Trans>No payload data</Trans>
          </div>
        ) : (
          <div className="border rounded-lg overflow-hidden">
            <div className="grid grid-cols-[80px_1fr_80px_100px] gap-2 px-3 py-2 bg-muted text-xs font-medium text-muted-foreground border-b">
              <div>
                <Trans>Method</Trans>
              </div>
              <div>URL</div>
              <div className="text-center">
                <Trans>Direction</Trans>
              </div>
              <div className="text-right">
                <Trans>Size</Trans>
              </div>
            </div>
            {report.largestPayloads.map((entry, i) => (
              <div
                key={`payload-${i}`}
                className="grid grid-cols-[80px_1fr_80px_100px] gap-2 px-3 py-2 text-xs border-b last:border-b-0 hover:bg-muted/50"
              >
                <div>
                  <Badge variant="outline" className="text-[10px] px-1.5">
                    {entry.method}
                  </Badge>
                </div>
                <div className="truncate" title={entry.url}>
                  {entry.url}
                </div>
                <div className="text-center">
                  <Badge
                    variant="outline"
                    className={`text-[10px] px-1.5 ${
                      entry.direction === "request"
                        ? "border-blue-300 text-blue-600"
                        : "border-green-300 text-green-600"
                    }`}
                  >
                    {entry.direction === "request" ? "REQ" : "RES"}
                  </Badge>
                </div>
                <div className="text-right font-mono font-medium">{formatBytes(entry.size)}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
