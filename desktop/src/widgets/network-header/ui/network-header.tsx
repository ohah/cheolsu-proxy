import { NetworkControls } from "./network-controls";
import { NetworkFilters } from "./network-filters";

interface NetworkHeaderProps {
  filterQueryString: string;
  appliedQueryString: string;
  filteredCount: number;
  totalCount: number;
  paused: boolean;
  togglePause: () => void;
  onFilterQueryChange: (query: string) => void;
  onApplyFilter: (query: string) => void;
  clearTransactions: () => void;
}

export function NetworkHeader({
  filterQueryString,
  appliedQueryString,
  filteredCount,
  totalCount,
  paused,
  togglePause,
  onFilterQueryChange,
  onApplyFilter,
  clearTransactions,
}: NetworkHeaderProps) {
  return (
    <div className="bg-sidebar w-full">
      <div className="flex items-center justify-between py-4 px-2 w-full">
        <div className="flex items-center gap-4 flex-1 w-full">
          <NetworkControls
            paused={paused}
            onTogglePause={togglePause}
            onClearTransactions={clearTransactions}
          />

          <NetworkFilters
            filterQueryString={filterQueryString}
            appliedQueryString={appliedQueryString}
            totalCount={totalCount}
            filteredCount={filteredCount}
            onFilterQueryChange={onFilterQueryChange}
            onApplyFilter={onApplyFilter}
          />
        </div>
      </div>
    </div>
  );
}
