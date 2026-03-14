import { memo } from "react";

import type { TableCellProps } from "../../model";

/** 응답 시간에 따른 바 색상 */
function getBarColor(ms: number): string {
  if (ms < 100) return "#4ade80"; // green — fast
  if (ms < 300) return "#a3e635"; // lime
  if (ms < 500) return "#facc15"; // yellow
  if (ms < 1000) return "#fb923c"; // orange
  return "#ef4444"; // red — slow
}

/** 응답 시간에 따른 바 폭 비율 (0~100) — 로그 스케일 */
function getBarWidth(ms: number): number {
  if (ms <= 0) return 0;
  // log10 스케일: 1ms=0%, 10ms~20%, 100ms~40%, 1000ms~60%, 10000ms~80%, 100000ms=100%
  const logVal = Math.log10(ms);
  const maxLog = 5; // 100000ms = 100s
  return Math.min(100, Math.max(2, (logVal / maxLog) * 100));
}

export const WaterfallCell = memo<TableCellProps>(({ data }) => {
  const { timeDiff, transaction } = data;
  const timing = transaction.response?.timing;

  const totalMs = timing?.total_ms ?? (typeof timeDiff === "number" ? timeDiff : 0);

  if (!totalMs || totalMs <= 0) {
    return <div className="text-sm text-muted-foreground">—</div>;
  }

  const barWidth = getBarWidth(totalMs);
  const barColor = getBarColor(totalMs);

  const tooltipLines: string[] = [`Total: ${totalMs}ms`];
  if (timing) {
    if (timing.dns_lookup_ms) tooltipLines.push(`DNS: ${timing.dns_lookup_ms}ms`);
    if (timing.tcp_connection_ms) tooltipLines.push(`TCP: ${timing.tcp_connection_ms}ms`);
    if (timing.tls_handshake_ms) tooltipLines.push(`TLS: ${timing.tls_handshake_ms}ms`);
    if (timing.ttfb_ms) tooltipLines.push(`TTFB: ${timing.ttfb_ms}ms`);
    if (timing.content_download_ms) tooltipLines.push(`Download: ${timing.content_download_ms}ms`);
  }

  return (
    <div className="flex items-center gap-1.5 h-full" title={tooltipLines.join("\n")}>
      <div className="flex-1 h-2.5 bg-muted/20 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all"
          style={{
            width: `${barWidth}%`,
            backgroundColor: barColor,
          }}
        />
      </div>
      <span className="text-[10px] font-mono text-muted-foreground shrink-0 w-14 text-right">
        {totalMs >= 1000 ? `${(totalMs / 1000).toFixed(1)}s` : `${totalMs}ms`}
      </span>
    </div>
  );
});

WaterfallCell.displayName = "WaterfallCell";
