import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Plus, X } from "lucide-react";
import { Input, Button } from "@/shared/ui";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";

export function ClientTagsSection() {
  const { t } = useLingui();
  const clientTags = useAppSettingsStore((s) => s.clientTags);
  const setClientTags = useAppSettingsStore((s) => s.setClientTags);

  const [newIp, setNewIp] = useState("");
  const [newLabel, setNewLabel] = useState("");

  const handleAdd = useCallback(() => {
    const ip = newIp.trim();
    const label = newLabel.trim();
    if (!ip || !label) return;
    setClientTags({ ...clientTags, [ip]: label });
    setNewIp("");
    setNewLabel("");
  }, [newIp, newLabel, clientTags, setClientTags]);

  const handleRemove = useCallback(
    (ip: string) => {
      const next = { ...clientTags };
      delete next[ip];
      setClientTags(next);
    },
    [clientTags, setClientTags],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  const entries = Object.entries(clientTags);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Client Tags</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>
            Assign labels to client IP addresses for easier identification and filtering
          </Trans>
        </p>
      </div>

      {entries.length > 0 && (
        <div className="space-y-2">
          {entries.map(([ip, label]) => (
            <div key={ip} className="flex items-center gap-2 group">
              <code className="text-xs font-mono bg-muted px-2 py-1.5 rounded min-w-[140px]">
                {ip}
              </code>
              <span className="text-sm text-muted-foreground">&rarr;</span>
              <span className="text-sm flex-1">{label}</span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 w-7 p-0 opacity-0 group-hover:opacity-100 transition-opacity"
                onClick={() => handleRemove(ip)}
              >
                <X className="w-3.5 h-3.5" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className="flex items-end gap-2">
        <div className="flex-1">
          <label className="text-sm font-medium mb-1.5 block">
            <Trans>IP Address</Trans>
          </label>
          <Input
            placeholder={t`e.g. 192.168.1.100`}
            value={newIp}
            onChange={(e) => setNewIp(e.target.value)}
            onKeyDown={handleKeyDown}
            className="font-mono text-xs"
          />
        </div>
        <div className="flex-1">
          <label className="text-sm font-medium mb-1.5 block">
            <Trans>Label</Trans>
          </label>
          <Input
            placeholder={t`e.g. iPhone, Android, QA Team`}
            value={newLabel}
            onChange={(e) => setNewLabel(e.target.value)}
            onKeyDown={handleKeyDown}
            className="text-xs"
          />
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-9"
          onClick={handleAdd}
          disabled={!newIp.trim() || !newLabel.trim()}
        >
          <Plus className="w-4 h-4 mr-1" />
          <Trans>Add</Trans>
        </Button>
      </div>

      <p className="text-xs text-muted-foreground">
        <Trans>Use tags in the filter with client="tag_name" to filter traffic by device</Trans>
      </p>
    </div>
  );
}
