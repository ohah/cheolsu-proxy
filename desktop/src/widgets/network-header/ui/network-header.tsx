import type { ReactNode } from "react";

import { NetworkControls } from "./network-controls";
import { NetworkFilters } from "./network-filters";

interface NetworkHeaderProps {
  paused: boolean;
  hasTransactions: boolean;
  exporting: boolean;
  togglePause: () => void;
  clearTransactions: () => void;
  onExportHar: () => void;
  onSaveSession: () => void;
  onLoadSession: () => void;
  onImportHar: () => void;
  onCompose?: () => void;
  filterSlot: ReactNode;
}

export function NetworkHeader({
  paused,
  hasTransactions,
  exporting,
  togglePause,
  clearTransactions,
  onExportHar,
  onSaveSession,
  onLoadSession,
  onImportHar,
  onCompose,
  filterSlot,
}: NetworkHeaderProps) {
  return (
    <div className="bg-sidebar w-full">
      <div className="flex items-center justify-between py-4 px-2 w-full">
        <div className="flex items-center gap-4 flex-1 w-full">
          <NetworkControls
            paused={paused}
            hasTransactions={hasTransactions}
            exporting={exporting}
            onTogglePause={togglePause}
            onClearTransactions={clearTransactions}
            onExportHar={onExportHar}
            onSaveSession={onSaveSession}
            onLoadSession={onLoadSession}
            onImportHar={onImportHar}
            onCompose={onCompose}
          />

          <NetworkFilters>{filterSlot}</NetworkFilters>
        </div>
      </div>
    </div>
  );
}
