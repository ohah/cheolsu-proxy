import { Pause, Play, Trash2 } from 'lucide-react';

import { Button } from '@/shared/ui';

interface NetworkControlsProps {
  paused: boolean;
  onTogglePause: () => void;
  onClearTransactions: () => void;
}

export const NetworkControls = ({ paused, onTogglePause, onClearTransactions }: NetworkControlsProps) => {
  return (
    <div className="flex items-center gap-2">
      <Button
        size="sm"
        variant="outline"
        onClick={onTogglePause}
        title={paused ? 'Resume recording' : 'Pause recording'}
      >
        {paused ? <Play className="w-4 h-4" /> : <Pause className="w-4 h-4" />}
      </Button>

      <Button size="sm" variant="outline" onClick={onClearTransactions} title="Clear all transactions">
        <Trash2 className="w-4 h-4" />
      </Button>
    </div>
  );
};
