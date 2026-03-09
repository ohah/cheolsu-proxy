import { useState } from "react";
import { Download, FolderOpen, Pause, PenLine, Play, Save, Trash2, Upload } from "lucide-react";
import { useLingui } from "@lingui/react/macro";

import { Button } from "@/shared/ui";
import { ReplayDialog } from "@/features/transaction-details/ui/replay-dialog";

interface NetworkControlsProps {
  paused: boolean;
  hasTransactions: boolean;
  exporting: boolean;
  onTogglePause: () => void;
  onClearTransactions: () => void;
  onExportHar: () => void;
  onSaveSession: () => void;
  onLoadSession: () => void;
  onImportHar: () => void;
}

export const NetworkControls = ({
  paused,
  hasTransactions,
  exporting,
  onTogglePause,
  onClearTransactions,
  onExportHar,
  onSaveSession,
  onLoadSession,
  onImportHar,
}: NetworkControlsProps) => {
  const { t } = useLingui();
  const [composeOpen, setComposeOpen] = useState(false);

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

      <div className="w-px h-5 bg-border" />

      <Button
        size="sm"
        variant="outline"
        onClick={() => setComposeOpen(true)}
        title={t`Compose request`}
      >
        <PenLine className="w-4 h-4" />
      </Button>
      <ReplayDialog open={composeOpen} onOpenChange={setComposeOpen} hideTrigger />

      <div className="w-px h-5 bg-border" />

      <Button
        size="sm"
        variant="outline"
        onClick={onSaveSession}
        disabled={!hasTransactions || exporting}
        title={t`Save Session`}
      >
        <Save className="w-4 h-4" />
      </Button>

      <Button size="sm" variant="outline" onClick={onLoadSession} title={t`Load Session`}>
        <FolderOpen className="w-4 h-4" />
      </Button>

      <div className="w-px h-5 bg-border" />

      <Button
        size="sm"
        variant="outline"
        onClick={onExportHar}
        disabled={!hasTransactions || exporting}
        title={t`Export HAR`}
      >
        <Download className="w-4 h-4" />
      </Button>

      <Button size="sm" variant="outline" onClick={onImportHar} title={t`Import HAR`}>
        <Upload className="w-4 h-4" />
      </Button>
    </div>
  );
};
