import { memo } from "react";

import type { TableCellProps } from "../../model";

export const ClientCell = memo<TableCellProps>(({ data }) => {
  const clientAddr = data.transaction.request?.client_addr;

  if (!clientAddr) {
    return <div className="text-sm text-muted-foreground">-</div>;
  }

  // IP:port에서 IP 부분만 표시
  const display = clientAddr.replace(/:\d+$/, "");

  return (
    <div className="text-sm font-mono truncate" title={clientAddr}>
      {display}
    </div>
  );
});

ClientCell.displayName = "ClientCell";
