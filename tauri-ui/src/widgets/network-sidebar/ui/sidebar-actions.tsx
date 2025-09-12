import { Filter, Settings } from 'lucide-react';

import { Button } from '@/shared/ui';

interface SidebarActionsProps {
  onFiltersClick: () => void;
  onSettingsClick: () => void;
}

export const SidebarActions = ({ onFiltersClick, onSettingsClick }: SidebarActionsProps) => {
  return (
    <div className="mt-6 pt-4 border-t border-sidebar-border space-y-1">
      <Button
        variant="ghost"
        className="w-full justify-start gap-3 text-sidebar-foreground hover:bg-sidebar-accent/50"
        onClick={onFiltersClick}
      >
        <Filter className="w-4 h-4" />
        <span>Filters</span>
      </Button>

      <Button
        variant="ghost"
        className="w-full justify-start gap-3 text-sidebar-foreground hover:bg-sidebar-accent/50"
        onClick={onSettingsClick}
      >
        <Settings className="w-4 h-4" />
        <span>Settings</span>
      </Button>
    </div>
  );
};
