import { Badge } from '@/shared/ui';

import { getStatusColor } from '../../lib';
import type { TableCellProps } from '../../model';

export const StatusCell = ({ data }: TableCellProps) => {
  const status = data.transaction.response?.status || 0;

  return (
    <div className="col-span-1">
      <Badge variant="outline" className={`text-xs ${getStatusColor(status)}`}>
        {status}
      </Badge>
    </div>
  );
};
