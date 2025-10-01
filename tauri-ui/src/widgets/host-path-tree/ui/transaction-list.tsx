import { Button } from '@/shared/ui/button';
import { HttpTransaction } from '@/entities/proxy';

import { getStatusDisplay, getStatusColor } from '../lib';
import { HostNode } from '../model';

interface TransactionListProps {
  transactions: HttpTransaction[];
  node: HostNode;
  depth: number;
  onTransactionSelect: (transaction: HttpTransaction) => void;
  selectedTransaction: HttpTransaction | null;
}

export const TransactionList = ({
  transactions,
  node,
  depth,
  onTransactionSelect,
  selectedTransaction,
}: TransactionListProps) => {
  const getTransactionKey = (transaction: HttpTransaction, index: number) => {
    return transaction.request?.id || `transaction-${index}`;
  };

  const isSelected = (transaction: HttpTransaction) => {
    return selectedTransaction?.request?.id === transaction.request?.id;
  };

  const getDisplayText = (transaction: HttpTransaction) => {
    return node.type === 'host' ? transaction.request?.uri : node.name;
  };

  return (
    <div>
      {transactions.map((transaction, index) => (
        <Button
          key={getTransactionKey(transaction, index)}
          variant="ghost"
          size="sm"
          className={`w-full justify-start h-7 text-xs font-mono hover:bg-muted/70 ${
            isSelected(transaction) ? 'bg-accent text-accent-foreground' : ''
          }`}
          style={{ paddingLeft: `${(depth + 1) * 16 + 24}px` }}
          onClick={() => onTransactionSelect(transaction)}
        >
          <span
            className={`inline-block w-12 text-center rounded px-1 text-xs font-medium mr-2 ${getStatusColor(transaction)}`}
          >
            {transaction.request?.method || '?'}
          </span>
          <span className="truncate text-xs">{getDisplayText(transaction)}</span>
          <span className="ml-auto text-xs text-muted-foreground">{getStatusDisplay(transaction)}</span>
        </Button>
      ))}
    </div>
  );
};
