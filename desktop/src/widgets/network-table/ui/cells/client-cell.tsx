import { memo } from "react";
import { Monitor, Smartphone } from "lucide-react";

import { extractIp } from "@/shared/lib";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import type { TableCellProps } from "../../model";

const LOCALHOST_ADDRS = new Set(["127.0.0.1", "::1", "localhost"]);

function isRemoteClient(ip: string): boolean {
  return !LOCALHOST_ADDRS.has(ip);
}

export const ClientCell = memo<TableCellProps>(({ data }) => {
  const clientAddr = data.transaction.request?.client_addr;
  const clientTags = useAppSettingsStore((s) => s.clientTags);

  if (!clientAddr) {
    return <div className="text-sm text-muted-foreground">-</div>;
  }

  const ip = extractIp(clientAddr);
  const tag = clientTags[ip];
  const remote = isRemoteClient(ip);

  return (
    <div className="flex items-center gap-1.5 text-sm font-mono truncate" title={clientAddr}>
      {remote ? (
        <Smartphone className="w-3 h-3 text-blue-500 shrink-0" />
      ) : (
        <Monitor className="w-3 h-3 text-muted-foreground shrink-0" />
      )}
      <span className="truncate">{tag || ip}</span>
    </div>
  );
});

ClientCell.displayName = "ClientCell";
