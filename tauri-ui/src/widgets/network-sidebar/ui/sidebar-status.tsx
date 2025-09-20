interface SidebarStatusProps {
  collapsed: boolean;
  isConnected: boolean;
  version?: string;
}

export const SidebarStatus = ({ collapsed, isConnected, version }: SidebarStatusProps) => {
  if (collapsed) {
    return null;
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
