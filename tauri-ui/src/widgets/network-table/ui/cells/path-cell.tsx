import { memo } from 'react';

import type { TableCellProps } from '../../model';

export const PathCell = memo<TableCellProps>(({ data }) => {
  const { authority, pathname } = data;

  return (
    <div className="col-span-5 flex flex-col gap-1">
      <div className="font-mono text-sm truncate" title={authority}>
        {authority}
      </div>
      <div className="font-mono text-sm truncate text-gray-500" title={pathname}>
        {pathname}
      </div>
    </div>
  );
});

PathCell.displayName = 'PathCell';
