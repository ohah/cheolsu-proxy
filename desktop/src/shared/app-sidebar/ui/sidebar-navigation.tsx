import { memo } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useLingui } from "@lingui/react";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { SIDEBAR_SECTIONS } from "../model";
import type { SidebarSection } from "../model";

interface SidebarNavigationProps {
  collapsed: boolean;
}

export const SidebarNavigation = memo(({ collapsed }: SidebarNavigationProps) => {
  const navigate = useNavigate();
  const location = useLocation();
  const { _ } = useLingui();

  // 현재 경로를 기반으로 활성 섹션 결정
  const getActiveSection = () => {
    switch (location.pathname) {
      case "/":
      case "/dashboard":
        return "network";
      case "/intercept-rules":
        return "intercept-rules";
      case "/map-rules":
        return "map-rules";
      case "/websocket":
        return "websocket";
      case "/server-replay":
        return "server-replay";
      case "/script":
        return "script";
      case "/settings":
        return "settings";
      default:
        return "network";
    }
  };

  const activeSection = getActiveSection();

  const handleSectionClick = (section: SidebarSection) => {
    switch (section.id) {
      case "network":
        navigate("/dashboard");
        break;
      case "intercept-rules":
        navigate("/intercept-rules");
        break;
      case "map-rules":
        navigate("/map-rules");
        break;
      case "websocket":
        navigate("/websocket");
        break;
      case "server-replay":
        navigate("/server-replay");
        break;
      case "script":
        navigate("/script");
        break;
      case "settings":
        navigate("/settings");
        break;
      default:
        navigate("/dashboard");
    }
  };

  return (
    <div className="space-y-1.5">
      {SIDEBAR_SECTIONS.map((section: SidebarSection) => {
        const Icon = section.icon;
        const isActive = activeSection === section.id;

        const button = (
          <Button
            key={section.id}
            variant="ghost"
            className={`w-full justify-start gap-3 hover:!bg-accent text-accent-foreground hover:!text-accent-foreground ${
              isActive
                ? "bg-accent text-accent-foreground dark:hover:bg-accent/50"
                : "text-sidebar-foreground hover:bg-sidebar-accent/50"
            }`}
            onClick={() => handleSectionClick(section)}
          >
            <Icon className="w-4 h-4" />
            {collapsed ? null : <span className="flex-1 text-left">{_(section.label)}</span>}
          </Button>
        );

        if (collapsed) {
          return (
            <Tooltip key={section.id}>
              <TooltipTrigger render={<div />}>{button}</TooltipTrigger>
              <TooltipContent side="right" sideOffset={4}>
                {_(section.label)}
              </TooltipContent>
            </Tooltip>
          );
        }

        return button;
      })}
    </div>
  );
});

SidebarNavigation.displayName = "SidebarNavigation";
