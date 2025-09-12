import { useMemo } from 'react';

import { Badge } from '@/shared/ui';

interface NetworkStatsProps {
  totalCount: number;
  filteredCount: number;
}

export const NetworkStats = ({ totalCount, filteredCount }: NetworkStatsProps) => {
  const count = useMemo(() => {
    if (totalCount !== filteredCount) {
      return `${filteredCount} of ${totalCount} transactions`;
    }
    return `${totalCount} transactions`;
  }, [totalCount, filteredCount]);

  return (
    <div className="flex items-center gap-2">
      <Badge variant="secondary" className="text-xs">
        {count}
      </Badge>
    </div>
  );
};
