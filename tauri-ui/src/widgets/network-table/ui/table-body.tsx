import { useCallback, useMemo } from 'react';

import type { HttpTransaction } from '@/entities/proxy';
import { VirtualizedScrollArea } from '@/shared/ui';

import type { TableRowData } from '../model';

import { TableRow } from './table-row';

interface TableBodyProps {
  data: TableRowData[];
  createTransactionSelectHandler: (request: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: number) => () => void;
}

export const TableBody = ({ data, createTransactionSelectHandler, createTransactionDeleteHandler }: TableBodyProps) => {
  const rowHandlers = useMemo(() => {
    return data.map((rowData, index) => {
      const id = rowData.transaction.request?.time ?? index;
      return {
        onSelect: createTransactionSelectHandler(rowData.transaction),
        onDelete: createTransactionDeleteHandler(id),
      };
    });
  }, [data, createTransactionSelectHandler, createTransactionDeleteHandler]);

  const renderItem = useCallback(
    (index: number) => {
      const rowData = data[index];
      const handlers = rowHandlers[index];

      return <TableRow data={rowData} onSelect={handlers.onSelect} onDelete={handlers.onDelete} />;
    },
    [data, rowHandlers],
  );

  if (data.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <p className="text-muted-foreground">No transactions to display</p>
      </div>
    );
  }

  return (
    <VirtualizedScrollArea
      itemCount={data.length}
      itemSize={69}
      className="flex-1 min-h-0"
      renderItem={renderItem}
      overscan={30}
    />
  );
};
