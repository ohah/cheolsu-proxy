import { memo } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { useLingui } from "@lingui/react";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { SIDEBAR_SECTIONS, DEFAULT_ACTIVE_SECTION } from "../model";
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
    const pathname = location.pathname === "/" ? "/dashboard" : location.pathname;
    const matched = SIDEBAR_SECTIONS.find((s) => s.path === pathname);
    return matched?.id ?? DEFAULT_ACTIVE_SECTION;
  };

  const activeSection = getActiveSection();

  const handleSectionClick = (section: SidebarSection) => {
    navigate(section.path);
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
