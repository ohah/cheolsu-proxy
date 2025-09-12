import { SidebarHeader } from './sidebar-header';
import { SidebarNavigation } from './sidebar-navigation';
import { SidebarStatus } from './sidebar-status';

import { useSidebarNavigation } from '../hooks';

interface NetworkSidebarProps {
  isConnected?: boolean;
  version?: string;
}

export function NetworkSidebar({ isConnected = true, version }: NetworkSidebarProps) {
  const { activeSection, createSelectionChangeHandler } = useSidebarNavigation();

  return (
    <div className="w-64 bg-sidebar border-r border-sidebar-border flex flex-col shrink-0">
      <SidebarHeader />

      <div className="flex-1 p-2">
        <SidebarNavigation activeSection={activeSection} createSelectionChangeHandler={createSelectionChangeHandler} />
      </div>

      <SidebarStatus isConnected={isConnected} version={version} />
    </div>
  );
}
