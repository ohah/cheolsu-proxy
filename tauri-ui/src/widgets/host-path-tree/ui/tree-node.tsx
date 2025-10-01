import { memo, useMemo } from 'react';

import { HttpTransaction } from '@/entities/proxy';

import { sortTreeNodes } from '../lib';
import { HostNode } from '../model';
import { useNodeActions } from '../hooks';

import { NodeContent } from './node-content';
import { TransactionList } from './transaction-list';

interface TreeNodeProps {
  node: HostNode;
  depth?: number;
  expandedPaths: Set<string>;
  onToggleExpanded: (path: string) => void;
  onTransactionSelect: (transaction: HttpTransaction) => void;
  selectedTransaction: HttpTransaction | null;
}

const TreeNodeComponent = ({
  node,
  depth = 0,
  expandedPaths,
  onToggleExpanded,
  onTransactionSelect,
  selectedTransaction,
}: TreeNodeProps) => {
  const hasChildren = node.children.size > 0;
  const isExpanded = expandedPaths.has(node.path);
  const hasTransactions = node.transactions.length > 0;
  const shouldShowContent = isExpanded || node.name === 'root';

  const { handleNodeClick } = useNodeActions({
    node,
    hasChildren,
    hasTransactions,
    onToggleExpanded,
    onTransactionSelect,
  });

  const sortedChildren = useMemo(() => {
    return sortTreeNodes(Array.from(node.children.values()));
  }, [node.children]);

  return (
    <div className="cursor-pointer">
      {node.name !== 'root' && (
        <NodeContent
          node={node}
          depth={depth}
          hasChildren={hasChildren}
          isExpanded={isExpanded}
          hasTransactions={hasTransactions}
          onToggleExpanded={onToggleExpanded}
          onNodeClick={handleNodeClick}
        />
      )}

      {hasTransactions && shouldShowContent && (
        <TransactionList
          transactions={node.transactions}
          node={node}
          depth={depth}
          onTransactionSelect={onTransactionSelect}
          selectedTransaction={selectedTransaction}
        />
      )}

      {hasChildren && shouldShowContent && (
        <div>
          {sortedChildren.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              expandedPaths={expandedPaths}
              onToggleExpanded={onToggleExpanded}
              onTransactionSelect={onTransactionSelect}
              selectedTransaction={selectedTransaction}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const TreeNode = memo(TreeNodeComponent);
