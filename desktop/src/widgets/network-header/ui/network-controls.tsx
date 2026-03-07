import { useState } from "react";
import { Download, Pause, Play, Trash2 } from "lucide-react";
import { useLingui } from "@lingui/react/macro";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

import type { HttpTransaction } from "@/entities/proxy";
import { buildHarLog } from "@/features/har-export";
import { Button } from "@/shared/ui";

interface NetworkControlsProps {
  paused: boolean;
  transactions: HttpTransaction[];
  onTogglePause: () => void;
  onClearTransactions: () => void;
}

export const NetworkControls = ({
  paused,
  transactions,
  onTogglePause,
  onClearTransactions,
}: NetworkControlsProps) => {
  const { t } = useLingui();
  const [exporting, setExporting] = useState(false);

  const handleExportHar = async () => {
    if (transactions.length === 0 || exporting) return;

    try {
      setExporting(true);

      const filePath = await save({
        defaultPath: "traffic.har",
        filters: [{ name: "HAR", extensions: ["har"] }],
      });
      if (!filePath) return;

      const har = await buildHarLog(transactions);
      const content = JSON.stringify(har, null, 2);
      await invoke("export_har_file", { path: filePath, content });
    } catch (err) {
      console.error("HAR export failed:", err);
    } finally {
      setExporting(false);
    }
  };

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

      <Button
        size="sm"
        variant="outline"
        onClick={handleExportHar}
        disabled={transactions.length === 0 || exporting}
        title={t`Export HAR`}
      >
        <Download className="w-4 h-4" />
      </Button>
    </div>
  );
};
