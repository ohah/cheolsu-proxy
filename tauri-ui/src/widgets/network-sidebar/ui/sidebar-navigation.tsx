import { memo } from 'react';

import { Button } from '@/shared/ui';

import { SIDEBAR_SECTIONS } from '../model';
import type { SidebarSection } from '../model';

interface SidebarNavigationProps {
  activeSection: string;
  createSelectionChangeHandler: (id: string) => () => void;
}

export const SidebarNavigation = memo(({ activeSection, createSelectionChangeHandler }: SidebarNavigationProps) => {
  return (
    <div className="space-y-1">
      {SIDEBAR_SECTIONS.map((section: SidebarSection) => {
        const Icon = section.icon;
        const isActive = activeSection === section.id;

        const onChange = createSelectionChangeHandler(section.id);

        return (
          <Button
            key={section.id}
            variant={isActive ? 'secondary' : 'ghost'}
            className={`w-full justify-start gap-3 ${
              isActive
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-sidebar-foreground hover:bg-sidebar-accent/50'
            }`}
            onClick={onChange}
            title={section.description}
          >
            <Icon className="w-4 h-4" />
            <span className="flex-1 text-left">{section.label}</span>
          </Button>
        );
      })}
    </div>
  );
});

SidebarNavigation.displayName = 'SidebarNavigation';
