import { ChevronRight, ChevronDown } from 'lucide-react';

import { Button } from '@/shared/ui/button';

import { getNodeIcon } from '../lib';
import { HostNode } from '../model';

interface NodeContentProps {
  node: HostNode;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
  hasTransactions: boolean;
  onToggleExpanded: (path: string) => void;
  onNodeClick: () => void;
}

export const NodeContent = ({
  node,
  depth,
  hasChildren,
  isExpanded,
  hasTransactions,
  onToggleExpanded,
  onNodeClick,
}: NodeContentProps) => {
  const handleToggleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onToggleExpanded(node.path);
  };

  return (
    <div className="flex items-center group hover:bg-muted/50 rounded-sm">
      <div style={{ paddingLeft: `${depth * 16}px` }} className="flex items-center flex-1 py-1">
        {hasChildren ? (
          <Button variant="ghost" size="sm" className="h-5 w-5 p-0 hover:bg-muted mr-1" onClick={handleToggleClick}>
            {isExpanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
          </Button>
        ) : (
          <div className="w-6" />
        )}

        <div className="flex items-center gap-2 flex-1 min-w-0 cursor-pointer" onClick={onNodeClick}>
          {getNodeIcon(node)}
          <span className={`text-xs truncate ${node.type === 'host' ? 'font-medium' : ''}`}>{node.name}</span>
          {hasTransactions && (
            <span className="text-xs text-muted-foreground bg-muted px-1 rounded">{node.transactions.length}</span>
          )}
        </div>
      </div>
    </div>
  );
};
