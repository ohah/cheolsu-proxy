import type { HttpTransaction } from '@/entities/proxy';

import { TableHeader } from './table-header';
import { TableBody } from './table-body';

import { useTableData } from '../hooks';

interface NetworkTableProps {
  transactions: HttpTransaction[];
  selectedTransaction: HttpTransaction | null;
  createTransactionSelectHandler: (transaction: HttpTransaction) => () => void;
  createTransactionDeleteHandler: (id: number) => () => void;
}

export const NetworkTable = ({
  transactions,
  selectedTransaction,
  createTransactionSelectHandler,
  createTransactionDeleteHandler,
}: NetworkTableProps) => {
  const { tableData } = useTableData({ transactions, selectedTransaction });

  return (
    <div className="flex flex-col flex-1 h-full">
      <TableHeader />
      <TableBody
        data={tableData}
        createTransactionSelectHandler={createTransactionSelectHandler}
        createTransactionDeleteHandler={createTransactionDeleteHandler}
      />
    </div>
  );
};
