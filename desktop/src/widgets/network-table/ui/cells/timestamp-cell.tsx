import { memo } from "react";

import { formatTime, formatTimeFull } from "@/shared/lib/format-time";
import type { TableCellProps } from "../../model";

export const TimestampCell = memo<TableCellProps>(({ data }) => {
  const requestTime = data.requestTime;

  if (!requestTime) {
    return <div className="text-sm text-muted-foreground">-</div>;
  }

  return (
    <div className="text-sm font-mono truncate" title={formatTimeFull(requestTime)}>
      {formatTime(requestTime)}
    </div>
  );
});

TimestampCell.displayName = "TimestampCell";
