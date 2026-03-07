import { logo } from "@/shared/assets";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { Menu } from "lucide-react";

interface SidebarHeaderProps {
  collapsed: boolean;
  toggleCollapse: () => void;
}

export const SidebarHeader = ({ collapsed, toggleCollapse }: SidebarHeaderProps) => {
  if (collapsed) {
    return (
      <div className="p-3.5 border-b border-sidebar-border flex items-center justify-between">
        <div className="h-10 flex items-center">
          <Tooltip>
            <TooltipTrigger render={<div />}>
              <Button variant="ghost" className="w-full" onClick={toggleCollapse}>
                <Menu className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right" sideOffset={4}>
              Expand sidebar
            </TooltipContent>
          </Tooltip>
        </div>
      </div>
    );
  }

  return (
    <div className="p-3.5 border-b border-sidebar-border flex items-center justify-between">
      <div className="flex items-center gap-2 h-10 shrink-0">
        <div className="w-9 h-9 rounded-lg flex items-center justify-center">
          <img
            src={logo}
            alt="Cheolsu Proxy Logo"
            className="w-9 h-9 text-sidebar-primary-foreground"
          />
        </div>
        <div>
          <h1 className="font-semibold text-sidebar-foreground">Cheolsu Proxy</h1>
        </div>
      </div>
      <Button variant="ghost" onClick={toggleCollapse}>
        <Menu className="w-4 h-4" />
      </Button>
    </div>
  );
};
