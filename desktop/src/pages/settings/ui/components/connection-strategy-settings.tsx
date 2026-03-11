import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { updateConnectionStrategy, type ConnectionStrategy } from "@/shared/api/proxy";
import {
  Button,
  Badge,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from "@/shared/ui";

const STRATEGY_OPTIONS: { value: ConnectionStrategy; label: string; description: string }[] = [
  {
    value: "lazy",
    label: "Lazy",
    description: "Connects to the server only when needed (default, sequential sniffing)",
  },
  {
    value: "eager",
    label: "Eager",
    description: "Starts background server connection immediately after ClientHello detection",
  },
  {
    value: "eager_with_fallback",
    label: "Eager with Fallback",
    description: "Tries Eager first, falls back to Lazy on failure",
  },
];

export function ConnectionStrategySettings() {
  const { t } = useLingui();
  const connectionStrategy = useAppSettingsStore((s) => s.connectionStrategy);
  const setConnectionStrategy = useAppSettingsStore((s) => s.setConnectionStrategy);

  const [selectedStrategy, setSelectedStrategy] = useState<ConnectionStrategy>(connectionStrategy);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<"idle" | "saved" | "error">("idle");

  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus("idle");

    try {
      await updateConnectionStrategy(selectedStrategy);
      setConnectionStrategy(selectedStrategy);
      setStatus("saved");
      setTimeout(() => setStatus("idle"), 2000);
    } catch (e) {
      console.error("연결 전략 설정 실패:", e);
      setStatus("error");
    } finally {
      setSaving(false);
    }
  }, [selectedStrategy, setConnectionStrategy]);

  const currentOption = STRATEGY_OPTIONS.find((o) => o.value === selectedStrategy);

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
              if (v) setSelectedStrategy(v as ConnectionStrategy);
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

        {currentOption && (
          <p className="text-xs text-muted-foreground">
            {currentOption.value === "lazy" && (
              <Trans>Connects to the server only when needed (default, sequential sniffing)</Trans>
            )}
            {currentOption.value === "eager" && (
              <Trans>
                Starts background server connection immediately after ClientHello detection
              </Trans>
            )}
            {currentOption.value === "eager_with_fallback" && (
              <Trans>Tries Eager first, falls back to Lazy on failure</Trans>
            )}
          </p>
        )}
      </div>

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? t`Saving...` : t`Save`}
        </Button>
        {status === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {status === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — is the proxy running?</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}
