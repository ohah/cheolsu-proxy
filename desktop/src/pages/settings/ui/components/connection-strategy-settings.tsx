import { useState, useCallback, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { updateConnectionStrategy, type ConnectionStrategy } from "@/shared/api/proxy";
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from "@/shared/ui";
import { useSettingsSave } from "../settings-save-context";

const STRATEGY_OPTIONS: { value: ConnectionStrategy; label: string }[] = [
  { value: "lazy", label: "Lazy" },
  { value: "eager", label: "Eager" },
  { value: "eager_with_fallback", label: "Eager with Fallback" },
];

export function ConnectionStrategySettings() {
  const { registerSave, unregisterSave, markDirty } = useSettingsSave();
  const connectionStrategy = useAppSettingsStore((s) => s.connectionStrategy);
  const setConnectionStrategy = useAppSettingsStore((s) => s.setConnectionStrategy);

  const [selectedStrategy, setSelectedStrategy] = useState<ConnectionStrategy>(connectionStrategy);

  const handleSave = useCallback(async () => {
    await updateConnectionStrategy(selectedStrategy);
    setConnectionStrategy(selectedStrategy);
  }, [selectedStrategy, setConnectionStrategy]);

  useEffect(() => {
    registerSave("connectionStrategy", handleSave);
    return () => unregisterSave("connectionStrategy");
  }, [registerSave, unregisterSave, handleSave]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Connection Strategy</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>
            Controls when the proxy connects to upstream servers for certificate sniffing
          </Trans>
        </p>
      </div>

      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium mb-1.5 block">
            <Trans>Strategy</Trans>
          </label>
          <Select
            value={selectedStrategy}
            onValueChange={(v) => {
              if (v) {
                setSelectedStrategy(v as ConnectionStrategy);
                markDirty("connectionStrategy");
              }
            }}
          >
            <SelectTrigger className="w-72">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STRATEGY_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value} label={option.label}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <p className="text-xs text-muted-foreground">
          {selectedStrategy === "lazy" && (
            <Trans>Connects to the server only when needed (default, sequential sniffing)</Trans>
          )}
          {selectedStrategy === "eager" && (
            <Trans>
              Starts background server connection immediately after ClientHello detection
            </Trans>
          )}
          {selectedStrategy === "eager_with_fallback" && (
            <Trans>Tries Eager first, falls back to Lazy on failure</Trans>
          )}
        </p>
      </div>
    </div>
  );
}
