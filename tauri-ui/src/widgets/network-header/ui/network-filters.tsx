import { QueryFilterEditor } from "@/features/query-filter-editor";

interface NetworkFiltersProps {
  filterQueryString: string;
  appliedQueryString: string;
  totalCount: number;
  filteredCount: number;
  onFilterQueryChange: (query: string) => void;
  onApplyFilter: (query: string) => void;
}

export const NetworkFilters = ({
  filterQueryString,
  appliedQueryString,
  totalCount,
  filteredCount,
  onFilterQueryChange,
  onApplyFilter,
}: NetworkFiltersProps) => {
  return (
    <div className="flex items-center gap-3 flex-1 min-w-0 h-[36px] w-full">
      <QueryFilterEditor
        totalCount={totalCount}
        filteredCount={filteredCount}
        value={filterQueryString}
        appliedValue={appliedQueryString}
        onChange={onFilterQueryChange}
        onApply={onApplyFilter}
      />
    </div>
  );
};
