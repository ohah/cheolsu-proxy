import { memo } from 'react';

import { Badge } from '@/shared/ui';
import { getStatusColor } from '@/entities/transaction';

import type { TableCellProps } from '../../model';

export const StatusCell = memo<TableCellProps>(({ data }) => {
  const status = data.transaction.response?.status || 0;

  return (
    <div className="col-span-1">
      <Badge variant="outline" className={`text-xs ${getStatusColor(status)}`}>
        {status}
      </Badge>
    </div>
  );
});

StatusCell.displayName = 'StatusCell';
