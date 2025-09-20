import clsx from 'clsx';

import { SidebarHeader } from './sidebar-header';
import { SidebarNavigation } from './sidebar-navigation';
import { SidebarStatus } from './sidebar-status';

import { useSidebarCollapse, useSidebarNavigation } from '../hooks';

interface NetworkSidebarProps {
  isConnected?: boolean;
  version?: string;
}

export function NetworkSidebar({ isConnected = true, version }: NetworkSidebarProps) {
  const { activeSection, createSelectionChangeHandler } = useSidebarNavigation();
  const { collapsed, toggleCollapse } = useSidebarCollapse();

  return (
    <div
      className={clsx(
        collapsed ? 'w-18' : 'w-64',
        'bg-sidebar border-r border-sidebar-border',
        'flex flex-col shrink-0',
        'transition-all duration-300 ease-in-out',
      )}
    >
      <SidebarHeader collapsed={collapsed} toggleCollapse={toggleCollapse} />

      <div className="flex-1 p-4">
        <SidebarNavigation
          collapsed={collapsed}
          activeSection={activeSection}
          createSelectionChangeHandler={createSelectionChangeHandler}
        />
      </div>

      <SidebarStatus collapsed={collapsed} isConnected={isConnected} version={version} />
    </div>
  );
}
