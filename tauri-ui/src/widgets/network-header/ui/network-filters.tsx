import { Search } from 'lucide-react';

import { Input, MultiSelect } from '@/shared/ui';

import { HTTP_METHOD_OPTIONS, STATUS_OPTIONS } from '../model';

interface NetworkFiltersProps {
  searchQuery: string;
  onSearchQueryChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  onStatusFilterChange: React.Dispatch<React.SetStateAction<string[]>>;
  onMethodFilterChange: React.Dispatch<React.SetStateAction<string[]>>;
}

export const NetworkFilters = ({
  searchQuery,
  onSearchQueryChange,
  onMethodFilterChange,
  onStatusFilterChange,
}: NetworkFiltersProps) => {
  return (
    <div className="flex items-center gap-3 flex-1 justify-between">
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input
          placeholder="Search path..."
          value={searchQuery}
          onChange={onSearchQueryChange}
          className="pl-9 min-w-64 flex-1"
        />
      </div>

      <div className="flex gap-3">
        <div>
          <MultiSelect
            options={HTTP_METHOD_OPTIONS}
            onValueChange={onMethodFilterChange}
            placeholder="Methods"
            searchable={false}
            maxCount={1}
          />
        </div>

        <div>
          <MultiSelect
            options={STATUS_OPTIONS}
            onValueChange={onStatusFilterChange}
            placeholder="Status Codes"
            searchable={false}
            maxCount={1}
          />
        </div>
      </div>
    </div>
  );
};
