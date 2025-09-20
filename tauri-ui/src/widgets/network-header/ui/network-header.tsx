import { NetworkControls } from './network-controls';
import { NetworkFilters } from './network-filters';
import { NetworkStats } from './network-stats';

interface NetworkHeaderProps {
  searchQuery: string;
  filteredCount: number;
  totalCount: number;
  paused: boolean;
  togglePause: () => void;
  onSearchQueryChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  onStatusFilterChange: React.Dispatch<React.SetStateAction<string[]>>;
  onMethodFilterChange: React.Dispatch<React.SetStateAction<string[]>>;
  clearTransactions: () => void;
}

export function NetworkHeader({
  searchQuery,
  filteredCount,
  totalCount,
  paused,
  togglePause,
  onSearchQueryChange,
  onStatusFilterChange,
  onMethodFilterChange,
  clearTransactions,
}: NetworkHeaderProps) {
  return (
    <div className="border-b border-border bg-sidebar">
      <div className="flex items-center justify-between p-4">
        <div className="flex items-center gap-4 flex-1">
          <NetworkControls paused={paused} onTogglePause={togglePause} onClearTransactions={clearTransactions} />

          <NetworkFilters
            searchQuery={searchQuery}
            onSearchQueryChange={onSearchQueryChange}
            onStatusFilterChange={onStatusFilterChange}
            onMethodFilterChange={onMethodFilterChange}
          />
          <NetworkStats totalCount={totalCount} filteredCount={filteredCount} />
        </div>
      </div>
    </div>
  );
}
