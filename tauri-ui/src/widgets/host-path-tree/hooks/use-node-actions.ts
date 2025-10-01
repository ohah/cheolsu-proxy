import { useCallback } from 'react';

import { HttpTransaction } from '@/entities/proxy';

import { HostNode } from '../model';

interface UseNodeActionsProps {
  node: HostNode;
  hasChildren: boolean;
  hasTransactions: boolean;
  onToggleExpanded: (path: string) => void;
  onTransactionSelect: (transaction: HttpTransaction) => void;
}

export const useNodeActions = ({
  node,
  hasChildren,
  hasTransactions,
  onToggleExpanded,
  onTransactionSelect,
}: UseNodeActionsProps) => {
  const handleNodeClick = useCallback(() => {
    if (hasChildren) {
      onToggleExpanded(node.path);
      return;
    }

    if (hasTransactions) {
      onTransactionSelect(node.transactions[0]);
      return;
    }
  }, [node, hasChildren, hasTransactions, onToggleExpanded, onTransactionSelect]);

  return {
    handleNodeClick,
  };
};
