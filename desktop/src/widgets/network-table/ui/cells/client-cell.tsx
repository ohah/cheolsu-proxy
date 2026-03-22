import { memo } from "react";
import { Monitor, Smartphone, Tablet } from "lucide-react";

import { extractIp } from "@/shared/lib";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import type { TableCellProps } from "../../model";

type DeviceType = "mobile" | "tablet" | "desktop";

function detectDeviceType(userAgent: string): DeviceType {
  const ua = userAgent.toLowerCase();
  if (/ipad|tablet|kindle|playbook/.test(ua)) return "tablet";
  if (/android/.test(ua) && !/mobile/.test(ua)) return "tablet";
  if (/iphone|ipod|android.*mobile|windows phone|blackberry|opera mini|iemobile/.test(ua))
    return "mobile";
  return "desktop";
}

const DeviceIcon = ({ type }: { type: DeviceType }) => {
  switch (type) {
    case "mobile":
      return <Smartphone className="w-3 h-3 text-blue-500 shrink-0" />;
    case "tablet":
      return <Tablet className="w-3 h-3 text-purple-500 shrink-0" />;
    case "desktop":
      return <Monitor className="w-3 h-3 text-muted-foreground shrink-0" />;
  }
};

export const ClientCell = memo<TableCellProps>(({ data }) => {
  const clientAddr = data.transaction.request?.client_addr;
  const clientTags = useAppSettingsStore((s) => s.clientTags);

  if (!clientAddr) {
    return <div className="text-sm text-muted-foreground">-</div>;
  }

  const ip = extractIp(clientAddr);
  const tag = clientTags[ip];
  const userAgent = data.transaction.request?.headers["user-agent"] ?? "";
  const deviceType = userAgent ? detectDeviceType(userAgent) : null;

  return (
    <div
      className="flex items-center gap-1.5 text-sm font-mono truncate"
      title={`${clientAddr}${userAgent ? `\n${userAgent}` : ""}`}
    >
      {deviceType && <DeviceIcon type={deviceType} />}
      <span className="truncate">{tag || ip}</span>
    </div>
  );
});

ClientCell.displayName = "ClientCell";
