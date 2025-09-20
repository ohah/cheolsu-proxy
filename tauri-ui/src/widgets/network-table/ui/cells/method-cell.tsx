import { Badge } from '@/shared/ui';

import { getMethodColor } from '@/entities/transaction';
import type { TableCellProps } from '../../model';

export const MethodCell = ({ data }: TableCellProps) => {
  const method = data.transaction.request?.method || '';

  return (
    <div className="col-span-1">
      <Badge variant="outline" className={`text-xs ${getMethodColor(method)}`}>
        {method}
      </Badge>
    </div>
  );
};
