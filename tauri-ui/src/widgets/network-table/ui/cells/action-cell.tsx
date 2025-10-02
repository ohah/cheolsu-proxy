import { memo } from 'react';
import { Trash2 } from 'lucide-react';

import { Button } from '@/shared/ui';

interface ActionCellProps {
  onDelete?: () => void;
}

export const ActionCell = memo<ActionCellProps>(({ onDelete }) => {
  return (
    <div className="col-span-1">
      <Button variant="outline" size="sm" onClick={onDelete} title="Delete transaction">
        <Trash2 className="w-4 h-4" />
      </Button>
    </div>
  );
});

ActionCell.displayName = 'ActionCell';