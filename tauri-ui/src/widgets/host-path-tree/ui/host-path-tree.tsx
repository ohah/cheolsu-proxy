import { useCallback } from 'react';
import { Globe } from 'lucide-react';

import { ScrollArea } from '@/shared/ui/scroll-area';
import { HttpTransaction } from '@/entities/proxy';

import { useHostTree } from '../hooks';
import { TreeNode } from './tree-node';

interface HostPathTreeProps {
  transactions: HttpTransaction[];
  createTransactionSelectHandler: (transaction: HttpTransaction) => () => void;
  selectedTransaction: HttpTransaction | null;
}

export function HostPathTree({ transactions, createTransactionSelectHandler, selectedTransaction }: HostPathTreeProps) {
  const { tree, expandedPaths, toggleExpanded } = useHostTree(transactions);

  const handleTransactionSelect = useCallback(
    (transaction: HttpTransaction) => {
      const handler = createTransactionSelectHandler(transaction);
      handler();
    },
    [createTransactionSelectHandler],
  );

  return (
    <div className="w-full h-full border-r border-border bg-card min-w-300px">
      <div className="p-3 border-b border-border">
        <h3 className="text-sm font-medium flex items-center gap-2">
          <Globe className="h-4 w-4" />
          Host Tree
        </h3>
      </div>
      <ScrollArea className="flex-1 p-2 h-[calc(100%-45px)] overflow-y-hidden">
        <TreeNode
          node={tree}
          expandedPaths={expandedPaths}
          onToggleExpanded={toggleExpanded}
          onTransactionSelect={handleTransactionSelect}
          selectedTransaction={selectedTransaction}
        />
      </ScrollArea>
    </div>
  );
}
