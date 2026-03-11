import { memo } from "react";

import { Badge } from "@/shared/ui";
import { getStatusColor } from "@/entities/transaction";

import type { TableCellProps } from "../../model";

export const StatusCell = memo<TableCellProps>(({ data }) => {
  const method = data.transaction.request?.method;
  const status = data.transaction.response?.status || 0;
  const isConnect = method === "CONNECT";

  return (
    <div>
      <Badge variant="outline" className={`text-xs ${getStatusColor(status)}`}>
        {isConnect ? "TUNNEL" : status}
      </Badge>
    </div>
  );
});

StatusCell.displayName = "StatusCell";
