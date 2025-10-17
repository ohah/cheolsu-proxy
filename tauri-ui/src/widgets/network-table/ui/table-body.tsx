import { useCallback, useMemo } from 'react';

import type { HttpTransaction } from '@/entities/proxy';
import { VirtualizedScrollArea } from '@/shared/ui';

import type { TableRowData } from '../model';

import { TableRow } from './table-row';

interface TableBodyProps {
  data: TableRowData[];
  pinnedTransactionIds: Set<string>;
  createTransactionSelectHandler: (request: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: string) => () => void;
  createTransactionPinHandler: (id: string) => () => void;
  isPinnedSection: boolean;
}

export const TableBody = ({
  data,
  pinnedTransactionIds,
  createTransactionSelectHandler,
  createTransactionDeleteHandler,
  createTransactionPinHandler,
  isPinnedSection,
}: TableBodyProps) => {
  const rowHandlers = useMemo(() => {
    return data.map((rowData) => {
      const id = rowData.transaction.request?.id ?? '';
      return {
        onSelect: createTransactionSelectHandler(rowData.transaction),
        onDelete: createTransactionDeleteHandler(id),
        onPin: createTransactionPinHandler(id),
        isPinned: pinnedTransactionIds.has(id),
      };
    });
  }, [data, createTransactionSelectHandler, createTransactionDeleteHandler]);

  const renderItem = useCallback(
    (index: number) => {
      const rowData = data[index];
      const handlers = rowHandlers[index];

      return (
        <TableRow
          data={rowData}
          onSelect={handlers.onSelect}
          onDelete={handlers.onDelete}
          onPin={handlers.onPin}
          isPinned={handlers.isPinned}
        />
      );
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

  if (isPinnedSection) {
    return (
      <div className="overflow-hidden">
        {data.map((_, index) => (
          <div key={data[index].transaction.request?.id ?? index}>{renderItem(index)}</div>
        ))}
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
