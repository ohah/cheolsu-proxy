import { Download, FolderOpen, Pause, PenLine, Play, Save, Trash2, Upload } from "lucide-react";
import { Trans } from "@lingui/react/macro";

import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";

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
  onCompose?: () => void;
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
  onCompose,
}: NetworkControlsProps) => {
  return (
    <div className="flex items-center gap-2">
      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button
            size="sm"
            variant="outline"
            onClick={onTogglePause}
          >
            {paused ? <Play className="w-4 h-4" /> : <Pause className="w-4 h-4" />}
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          {paused ? <Trans>Resume recording</Trans> : <Trans>Pause recording</Trans>}
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button
            size="sm"
            variant="outline"
            onClick={onClearTransactions}
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Clear all transactions</Trans>
        </TooltipContent>
      </Tooltip>

      <div className="w-px h-5 bg-border" />

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button size="sm" variant="outline" onClick={() => onCompose?.()}>
            <PenLine className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Compose request</Trans>
        </TooltipContent>
      </Tooltip>

      <div className="w-px h-5 bg-border" />

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button
            size="sm"
            variant="outline"
            onClick={onSaveSession}
            disabled={!hasTransactions || exporting}
          >
            <Save className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Save Session</Trans>
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button size="sm" variant="outline" onClick={onLoadSession}>
            <FolderOpen className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Load Session</Trans>
        </TooltipContent>
      </Tooltip>

      <div className="w-px h-5 bg-border" />

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button
            size="sm"
            variant="outline"
            onClick={onExportHar}
            disabled={!hasTransactions || exporting}
          >
            <Download className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Export HAR</Trans>
        </TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger render={<div />}>
          <Button size="sm" variant="outline" onClick={onImportHar}>
            <Upload className="w-4 h-4" />
          </Button>
        </TooltipTrigger>
        <TooltipContent sideOffset={4}>
          <Trans>Import HAR</Trans>
        </TooltipContent>
      </Tooltip>
    </div>
  );
};
