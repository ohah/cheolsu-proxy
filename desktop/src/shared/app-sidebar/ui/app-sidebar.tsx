import { cn } from "@/shared/lib";
import { useProxyStore } from "@/shared/stores";

import { SidebarHeader } from "./sidebar-header";
import { SidebarMcp } from "./sidebar-mcp";
import { SidebarNavigation } from "./sidebar-navigation";
import { SidebarStatus } from "./sidebar-status";

import { useSidebarCollapse } from "../hooks";

export function AppSidebar() {
  const { isConnected } = useProxyStore();
  const { collapsed, toggleCollapse } = useSidebarCollapse();

  return (
    <div
      className={cn(
        collapsed ? "w-18" : "w-64",
        "bg-sidebar",
        "flex flex-col shrink-0",
        "transition-all duration-300 ease-in-out",
      )}
    >
      <SidebarHeader collapsed={collapsed} toggleCollapse={toggleCollapse} />

      <div className="flex-1 px-4 py-2">
        <SidebarNavigation collapsed={collapsed} />
      </div>

      <SidebarMcp collapsed={collapsed} />
      <SidebarStatus collapsed={collapsed} isConnected={isConnected} />
    </div>
  );
}
