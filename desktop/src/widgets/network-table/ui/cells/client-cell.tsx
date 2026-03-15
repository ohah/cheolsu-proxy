import { memo } from "react";

import type { TableCellProps } from "../../model";

export const ClientCell = memo<TableCellProps>(({ data }) => {
  const clientAddr = data.transaction.request?.client_addr;

  if (!clientAddr) {
    return <div className="text-sm text-muted-foreground">-</div>;
  }

  // IP:port에서 IP 부분만 표시 (IPv6 [::1]:port 대응)
  let display = clientAddr;
  if (clientAddr.startsWith("[")) {
    // IPv6: [::1]:port → ::1
    display = clientAddr.replace(/^\[(.+)\]:\d+$/, "$1");
  } else {
    // IPv4: 127.0.0.1:port → 127.0.0.1
    display = clientAddr.replace(/:\d+$/, "");
  }

  return (
    <div className="text-sm font-mono truncate" title={clientAddr}>
      {display}
    </div>
  );
});

ClientCell.displayName = "ClientCell";
