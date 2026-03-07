import { Circle } from "lucide-react";
import { Trans, useLingui } from "@lingui/react/macro";
import { cn } from "@/shared/lib";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";

interface SidebarStatusProps {
  collapsed: boolean;
  isConnected: boolean;
  version?: string;
}

export const SidebarStatus = ({ collapsed, isConnected, version }: SidebarStatusProps) => {
  const { t } = useLingui();
  if (collapsed) {
    return (
      <div className="p-4 border-t border-sidebar-border flex justify-center">
        <Tooltip>
          <TooltipTrigger render={<div />}>
            <Circle
              className={cn(
                "w-3 h-3",
                isConnected ? "text-green-500 fill-green-500" : "text-red-500 fill-red-500",
              )}
            />
          </TooltipTrigger>
          <TooltipContent side="right" sideOffset={4}>
            {isConnected ? t`Connected` : t`Disconnected`}
          </TooltipContent>
        </Tooltip>
      </div>
    );
  }

  return (
    <div className="p-4 border-t border-sidebar-border">
      <div className="text-xs text-muted-foreground space-y-1">
        <div className="flex justify-between">
          <span>
            <Trans>Status</Trans>
          </span>
          <span className={isConnected ? "text-green-600" : "text-red-600"}>
            {isConnected ? t`Connected` : t`Disconnected`}
          </span>
        </div>
        {version && (
          <div className="flex justify-between">
            <span>
              <Trans>Version</Trans>
            </span>
            <span>{version}</span>
          </div>
        )}
      </div>
    </div>
  );
};
