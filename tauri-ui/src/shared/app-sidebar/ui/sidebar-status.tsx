import { Circle } from 'lucide-react';
import { cn } from '@/shared/lib';

interface SidebarStatusProps {
  collapsed: boolean;
  isConnected: boolean;
  version?: string;
}

export const SidebarStatus = ({ collapsed, isConnected, version }: SidebarStatusProps) => {
  if (collapsed) {
    return (
      <div className="p-2 border-t border-sidebar-border flex justify-center">
        <div title={isConnected ? 'Connected' : 'Disconnected'}>
          <Circle
            className={cn('w-3 h-3', isConnected ? 'text-green-500 fill-green-500' : 'text-red-500 fill-red-500')}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 border-t border-sidebar-border">
      <div className="text-xs text-muted-foreground space-y-1">
        <div className="flex justify-between">
          <span>Status</span>
          <span className={isConnected ? 'text-green-600' : 'text-red-600'}>
            {isConnected ? 'Connected' : 'Disconnected'}
          </span>
        </div>
        {version && (
          <div className="flex justify-between">
            <span>Version</span>
            <span>{version}</span>
          </div>
        )}
      </div>
    </div>
  );
};
