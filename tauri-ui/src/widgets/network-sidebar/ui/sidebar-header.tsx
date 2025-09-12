import { logo } from '@/shared/assets';

export const SidebarHeader = () => {
  return (
    <div className="p-4 border-b border-sidebar-border">
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 bg-sidebar-primary rounded-lg flex items-center justify-center">
          <img src={logo} alt="Cheolsu Proxy Logo" className="w-8 h-8 text-sidebar-primary-foreground" />
        </div>
        <div>
          <h1 className="font-semibold text-sidebar-foreground">Cheolsu Proxy</h1>
          <p className="text-xs text-muted-foreground">Network Monitor</p>
        </div>
      </div>
    </div>
  );
};
