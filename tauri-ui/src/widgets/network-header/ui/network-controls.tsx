import { Pause, Play, Trash2 } from "lucide-react";
import { useLingui } from "@lingui/react/macro";

import { Button } from "@/shared/ui";

interface NetworkControlsProps {
  paused: boolean;
  onTogglePause: () => void;
  onClearTransactions: () => void;
}

export const NetworkControls = ({
  paused,
  onTogglePause,
  onClearTransactions,
}: NetworkControlsProps) => {
  const { t } = useLingui();

  return (
    <div className="flex items-center gap-2">
      <Button
        size="sm"
        variant="outline"
        onClick={onTogglePause}
        title={paused ? t`Resume recording` : t`Pause recording`}
      >
        {paused ? <Play className="w-4 h-4" /> : <Pause className="w-4 h-4" />}
      </Button>

      <Button
        size="sm"
        variant="outline"
        onClick={onClearTransactions}
        title={t`Clear all transactions`}
      >
        <Trash2 className="w-4 h-4" />
      </Button>
    </div>
  );
};
