import { memo } from "react";

import { formatBytes } from "../../lib";
import type { TableCellProps } from "../../model";

export const SizeCell = memo<TableCellProps>(({ data }) => {
  const requestSize = data.transaction.request?.body_size || 0;
  const responseSize = data.transaction.response?.body_size || 0;
  const totalSize = requestSize + responseSize;

  return (
    <div
      className="text-sm font-mono"
      title={`Request: ${formatBytes(requestSize)}, Response: ${formatBytes(responseSize)}`}
    >
      {formatBytes(totalSize)}
    </div>
  );
});

SizeCell.displayName = "SizeCell";
