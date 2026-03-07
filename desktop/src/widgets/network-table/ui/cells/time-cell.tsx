import { memo } from "react";

import type { TableCellProps } from "../../model";

export const TimeCell = memo<TableCellProps>(({ data }) => {
  const { timeDiff } = data;

  const displayTime = typeof timeDiff === "number" ? `${timeDiff}ms` : timeDiff;

  return (
    <div className="text-sm font-mono" title={`Response time: ${displayTime}`}>
      {displayTime}
    </div>
  );
});

TimeCell.displayName = "TimeCell";
