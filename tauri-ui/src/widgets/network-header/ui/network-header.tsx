'use client';

import { Input, Button, Badge, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/shared/ui';
import { MultiSelect } from '@/shared/ui/multi-select';
import { Search, RefreshCw, Download, Trash2, Play, Square } from 'lucide-react';
import { HTTP_METHOD_OPTIONS, STATUS_OPTIONS } from '../model';

interface NetworkHeaderProps {
  filter: string;
  onFilterChange: (filter: string) => void;
  statusFilter: string[];
  onStatusFilterChange: (filter: string[]) => void;
  methodFilter: string[];
  onMethodFilterChange: (filter: string[]) => void;
  requestCount: number;
  paused: boolean;
  setPaused: (paused: boolean) => void;
  clearRequests: () => void;
}

export function NetworkHeader({
  paused,
  setPaused,
  clearRequests,
  filter,
  onFilterChange,
  statusFilter,
  onStatusFilterChange,
  methodFilter,
  onMethodFilterChange,
  requestCount,
}: NetworkHeaderProps) {
  const handlePlayClick = () => {
    setPaused(false);
  };

  const handleStopClick = () => {
    setPaused(true);
  };

  return (
    <div className="border-b border-border bg-card">
      <div className="flex items-center justify-between p-4">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Button size="sm" variant={paused ? 'outline' : 'secondary'} onClick={handlePlayClick} disabled={!paused}>
              <Play className="w-4 h-4" />
            </Button>
            <Button size="sm" variant={paused ? 'destructive' : 'outline'} onClick={handleStopClick} disabled={paused}>
              <Square className="w-4 h-4" />
            </Button>
            <Button size="sm" variant="outline" onClick={clearRequests}>
              <Trash2 className="w-4 h-4" />
            </Button>
          </div>

          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-muted-foreground" />
              <Input
                placeholder="Filter requests..."
                value={filter}
                onChange={(e) => onFilterChange(e.target.value)}
                className="pl-9 w-64"
              />
            </div>

            <div className="flex items-center gap-2">
              <MultiSelect
                options={HTTP_METHOD_OPTIONS}
                onValueChange={onMethodFilterChange}
                placeholder="Select Methods"
                searchable={false}
                maxCount={1}
              />
            </div>
            <div className="flex items-center gap-2">
              <MultiSelect
                options={STATUS_OPTIONS}
                onValueChange={onStatusFilterChange}
                placeholder="Select Statuses"
                searchable={false}
                maxCount={1}
              />
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <Badge variant="secondary" className="text-xs">
            {requestCount} requests
          </Badge>
        </div>
      </div>
    </div>
  );
}
