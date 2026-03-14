import { memo, useMemo } from "react";

import type { TableCellProps } from "../../model";

const COLORS = {
  dns: "#4ade80", // green
  tcp: "#fb923c", // orange
  tls: "#a78bfa", // purple
  ttfb: "#60a5fa", // blue
  download: "#94a3b8", // gray
} as const;

export const WaterfallCell = memo<TableCellProps>(({ data }) => {
  const { timeDiff, transaction } = data;
  const timing = transaction.response?.timing;

  const totalMs = timing?.total_ms ?? (typeof timeDiff === "number" ? timeDiff : 0);

  const segments = useMemo(() => {
    if (!totalMs || totalMs <= 0) return [];

    if (timing) {
      const parts: { label: string; ms: number; color: string }[] = [];
      if (timing.dns_lookup_ms)
        parts.push({ label: "DNS", ms: timing.dns_lookup_ms, color: COLORS.dns });
      if (timing.tcp_connection_ms)
        parts.push({ label: "TCP", ms: timing.tcp_connection_ms, color: COLORS.tcp });
      if (timing.tls_handshake_ms)
        parts.push({ label: "TLS", ms: timing.tls_handshake_ms, color: COLORS.tls });
      if (timing.ttfb_ms) parts.push({ label: "TTFB", ms: timing.ttfb_ms, color: COLORS.ttfb });
      if (timing.content_download_ms)
        parts.push({ label: "Download", ms: timing.content_download_ms, color: COLORS.download });

      if (parts.length > 0) return parts;
    }

    // timing 정보가 없으면 전체를 하나의 바로 표시
    return [{ label: "Total", ms: totalMs, color: COLORS.ttfb }];
  }, [timing, totalMs]);

  if (!totalMs || segments.length === 0) {
    return <div className="text-sm text-muted-foreground">—</div>;
  }

  const tooltip = segments.map((s) => `${s.label}: ${s.ms}ms`).join("\n");

  return (
    <div className="flex items-center gap-1 h-full" title={tooltip}>
      <div className="flex-1 h-3 bg-muted/30 rounded-sm overflow-hidden flex">
        {segments.map((segment, i) => (
          <div
            key={i}
            className="h-full min-w-[2px]"
            style={{
              width: `${(segment.ms / totalMs) * 100}%`,
              backgroundColor: segment.color,
            }}
          />
        ))}
      </div>
      <span className="text-[10px] font-mono text-muted-foreground shrink-0 w-12 text-right">
        {totalMs}ms
      </span>
    </div>
  );
});

WaterfallCell.displayName = "WaterfallCell";
