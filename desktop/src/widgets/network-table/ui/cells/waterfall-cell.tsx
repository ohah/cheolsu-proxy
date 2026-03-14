import { memo } from "react";

import type { TableCellProps } from "../../model";

/** 응답 시간에 따른 바 색상 */
function getBarColor(ms: number): string {
  if (ms < 100) return "#4ade80"; // green
  if (ms < 300) return "#a3e635"; // lime
  if (ms < 500) return "#facc15"; // yellow
  if (ms < 1000) return "#fb923c"; // orange
  return "#ef4444"; // red
}

function formatTime(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.round(ms)}ms`;
}

export const WaterfallCell = memo<TableCellProps>(({ data }) => {
  const { transaction, timelineStart, timelineEnd } = data;
  const reqTime = transaction.request?.time;
  const resTime = transaction.response?.time;

  if (reqTime == null || timelineStart == null || timelineEnd == null) {
    return <div className="text-sm text-muted-foreground">—</div>;
  }

  const totalRange = timelineEnd - timelineStart;
  if (totalRange <= 0) {
    return <div className="text-sm text-muted-foreground">—</div>;
  }

  const endTime = resTime ?? reqTime;
  const durationNs = endTime - reqTime;
  const durationMs = Math.max(0, Math.trunc(durationNs / 1e6));

  // 시간축에서의 시작 위치와 폭 (%)
  const leftPct = ((reqTime - timelineStart) / totalRange) * 100;
  const widthPct = Math.max(0.5, (durationNs / totalRange) * 100);

  const barColor = getBarColor(durationMs);

  const tooltip = [
    `Start: +${formatTime(Math.trunc((reqTime - timelineStart) / 1e6))}`,
    `Duration: ${formatTime(durationMs)}`,
  ].join("\n");

  return (
    <div className="flex items-center h-full" title={tooltip}>
      <div className="relative flex-1 h-3 bg-muted/10 rounded-sm overflow-hidden">
        {/* 시간축 눈금 (25%, 50%, 75%) */}
        <div className="absolute inset-0 flex">
          <div className="w-1/4 border-r border-muted/20" />
          <div className="w-1/4 border-r border-muted/20" />
          <div className="w-1/4 border-r border-muted/20" />
        </div>
        {/* 요청 바 */}
        <div
          className="absolute top-0 h-full rounded-sm"
          style={{
            left: `${Math.min(leftPct, 99)}%`,
            width: `${Math.min(widthPct, 100 - leftPct)}%`,
            backgroundColor: barColor,
            minWidth: "2px",
          }}
        />
      </div>
    </div>
  );
});

WaterfallCell.displayName = "WaterfallCell";
