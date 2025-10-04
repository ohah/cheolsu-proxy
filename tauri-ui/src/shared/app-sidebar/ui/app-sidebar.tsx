import clsx from 'clsx';

import { SidebarHeader } from './sidebar-header';
import { SidebarNavigation } from './sidebar-navigation';
import { SidebarStatus } from './sidebar-status';

import { useSidebarCollapse } from '../hooks';

interface AppSidebarProps {
  isConnected?: boolean;
  version?: string;
}

export function AppSidebar({ isConnected = true, version }: AppSidebarProps) {
  const { collapsed, toggleCollapse } = useSidebarCollapse();

  return (
    <div
      className={clsx(
        collapsed ? 'w-18' : 'w-64',
        'bg-sidebar',
        'flex flex-col shrink-0',
        'transition-all duration-300 ease-in-out',
      )}
    >
      <SidebarHeader collapsed={collapsed} toggleCollapse={toggleCollapse} />

      <div className="flex-1 px-4 py-2">
        <SidebarNavigation collapsed={collapsed} />
      </div>

      <SidebarStatus collapsed={collapsed} isConnected={isConnected} version={version} />
    </div>
  );
}
