import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { updateThrottle, type ThrottleConfig } from "@/shared/api/proxy";
import { Button, Input, Switch, Badge, Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";

const THROTTLE_PRESETS = [
  { value: "none", label: "None", config: null },
  {
    value: "gprs",
    label: "GPRS (50 KB/s)",
    config: { enabled: true, download_rate: 50 * 1024, upload_rate: 20 * 1024, latency_ms: 500 },
  },
  {
    value: "slow3g",
    label: "Slow 3G (500 KB/s)",
    config: {
      enabled: true,
      download_rate: 500 * 1024,
      upload_rate: 500 * 1024,
      latency_ms: 400,
    },
  },
  {
    value: "fast3g",
    label: "Fast 3G (1.6 MB/s)",
    config: {
      enabled: true,
      download_rate: 1_600 * 1024,
      upload_rate: 768 * 1024,
      latency_ms: 150,
    },
  },
  {
    value: "lte",
    label: "4G/LTE (4 MB/s)",
    config: {
      enabled: true,
      download_rate: 4 * 1024 * 1024,
      upload_rate: 3 * 1024 * 1024,
      latency_ms: 50,
    },
  },
  {
    value: "wifi",
    label: "WiFi (30 MB/s)",
    config: {
      enabled: true,
      download_rate: 30 * 1024 * 1024,
      upload_rate: 15 * 1024 * 1024,
      latency_ms: 2,
    },
  },
  { value: "custom", label: "Custom", config: null },
] as const;

export function ThrottleSettings() {
  const { t } = useLingui();
  const throttleConfig = useAppSettingsStore((s) => s.throttleConfig);
  const setThrottleConfig = useAppSettingsStore((s) => s.setThrottleConfig);

  const [throttleEnabled, setThrottleEnabled] = useState(throttleConfig.enabled);
  const [throttlePreset, setThrottlePreset] = useState(throttleConfig.preset);
  const [throttleDownload, setThrottleDownload] = useState(throttleConfig.download);
  const [throttleUpload, setThrottleUpload] = useState(throttleConfig.upload);
  const [throttleLatency, setThrottleLatency] = useState(throttleConfig.latency);
  const [throttleSaving, setThrottleSaving] = useState(false);
  const [throttleStatus, setThrottleStatus] = useState<"idle" | "saved" | "error">("idle");

  const handleThrottleSave = useCallback(async () => {
    setThrottleSaving(true);
    setThrottleStatus("idle");

    try {
      let config: ThrottleConfig | null = null;

      if (throttleEnabled) {
        if (throttlePreset === "custom") {
          const dlRate = Number.parseInt(throttleDownload, 10);
          const ulRate = Number.parseInt(throttleUpload, 10);
          config = {
            enabled: true,
            download_rate: dlRate > 0 ? dlRate * 1024 : null, // KB/s -> bytes/s
            upload_rate: ulRate > 0 ? ulRate * 1024 : null,
            latency_ms: Number.parseInt(throttleLatency, 10) || 0,
          };
        } else {
          const preset = THROTTLE_PRESETS.find((p) => p.value === throttlePreset);
          if (preset?.config) {
            config = preset.config;
          }
        }
      }

      await updateThrottle(config);

      const localConfig = {
        enabled: throttleEnabled,
        preset: throttlePreset,
        download: throttleDownload,
        upload: throttleUpload,
        latency: throttleLatency,
      };
      setThrottleConfig(localConfig);

      setThrottleStatus("saved");
      setTimeout(() => setThrottleStatus("idle"), 2000);
    } catch (e) {
      console.error("스로틀링 설정 저장 실패:", e);
      setThrottleStatus("error");
    } finally {
      setThrottleSaving(false);
    }
  }, [throttleEnabled, throttlePreset, throttleDownload, throttleUpload, throttleLatency, setThrottleConfig]);

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Network Throttling</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Simulate slow network conditions for testing</Trans>
          </p>
        </div>
        <Switch checked={throttleEnabled} onCheckedChange={setThrottleEnabled} />
      </div>

      {throttleEnabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Profile</Trans>
            </label>
            <Select
              value={throttlePreset}
              onValueChange={(v) => {
                if (v) setThrottlePreset(v);
              }}
            >
              <SelectTrigger className="w-64">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THROTTLE_PRESETS.map((preset) => (
                  <SelectItem key={preset.value} value={preset.value}>
                    {preset.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {throttlePreset === "custom" && (
            <div className="space-y-3">
              <div className="flex gap-3">
                <div className="flex-1">
                  <label className="text-sm font-medium mb-1.5 block">
                    <Trans>Download (KB/s)</Trans>
                  </label>
                  <Input
                    type="number"
                    placeholder={t`Unlimited`}
                    value={throttleDownload}
                    onChange={(e) => setThrottleDownload(e.target.value)}
                  />
                </div>
                <div className="flex-1">
                  <label className="text-sm font-medium mb-1.5 block">
                    <Trans>Upload (KB/s)</Trans>
                  </label>
                  <Input
                    type="number"
                    placeholder={t`Unlimited`}
                    value={throttleUpload}
                    onChange={(e) => setThrottleUpload(e.target.value)}
                  />
                </div>
                <div className="w-28">
                  <label className="text-sm font-medium mb-1.5 block">
                    <Trans>Latency (ms)</Trans>
                  </label>
                  <Input
                    type="number"
                    placeholder="0"
                    value={throttleLatency}
                    onChange={(e) => setThrottleLatency(e.target.value)}
                  />
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={handleThrottleSave} disabled={throttleSaving}>
          {throttleSaving ? t`Saving...` : t`Save`}
        </Button>
        {throttleStatus === "saved" && (
          <Badge variant="outline" className="text-green-600 border-green-600">
            <Trans>Saved</Trans>
          </Badge>
        )}
        {throttleStatus === "error" && (
          <Badge variant="outline" className="text-red-600 border-red-600">
            <Trans>Failed — is the proxy running?</Trans>
          </Badge>
        )}
      </div>
    </div>
  );
}
