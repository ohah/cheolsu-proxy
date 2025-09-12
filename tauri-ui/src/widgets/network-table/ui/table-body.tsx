import type { HttpTransaction } from '@/entities/proxy';

import { TableRow } from './table-row';

import type { TableRowData } from '../model';

interface TableBodyProps {
  data: TableRowData[];
  createTransactionSelectHandler: (request: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: number) => () => void;
}

export function TableBody({ data, createTransactionSelectHandler, createTransactionDeleteHandler }: TableBodyProps) {
  if (data.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center p-8">
        <p className="text-muted-foreground">No transactions to display</p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto">
      {data.map((rowData) => {
        const id = rowData.transaction.request?.time ?? rowData.index;

        const onSelect = createTransactionSelectHandler(rowData.transaction);
        const onDelete = createTransactionDeleteHandler(id);

        return <TableRow key={id} data={rowData} onSelect={onSelect} onDelete={onDelete} />;
      })}
    </div>
  );
}
