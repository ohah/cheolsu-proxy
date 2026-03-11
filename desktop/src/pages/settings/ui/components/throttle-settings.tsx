import { useState, useCallback, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useAppSettingsStore } from "@/shared/stores/app-settings-store";
import { updateThrottle, type ThrottleConfig } from "@/shared/api/proxy";
import {
  Input,
  Switch,
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from "@/shared/ui";
import { useSettingsSave } from "../settings-save-context";

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
  const { registerSave, unregisterSave, markDirty } = useSettingsSave();
  const throttleConfig = useAppSettingsStore((s) => s.throttleConfig);
  const setThrottleConfig = useAppSettingsStore((s) => s.setThrottleConfig);

  const [throttleEnabled, setThrottleEnabled] = useState(throttleConfig.enabled);
  const [throttlePreset, setThrottlePreset] = useState(throttleConfig.preset);
  const [throttleDownload, setThrottleDownload] = useState(throttleConfig.download);
  const [throttleUpload, setThrottleUpload] = useState(throttleConfig.upload);
  const [throttleLatency, setThrottleLatency] = useState(throttleConfig.latency);

  const handleSave = useCallback(async () => {
    let config: ThrottleConfig | null = null;

    if (throttleEnabled) {
      if (throttlePreset === "custom") {
        const dlRate = Number.parseInt(throttleDownload, 10);
        const ulRate = Number.parseInt(throttleUpload, 10);
        config = {
          enabled: true,
          download_rate: dlRate > 0 ? dlRate * 1024 : null,
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

    setThrottleConfig({
      enabled: throttleEnabled,
      preset: throttlePreset,
      download: throttleDownload,
      upload: throttleUpload,
      latency: throttleLatency,
    });
  }, [throttleEnabled, throttlePreset, throttleDownload, throttleUpload, throttleLatency, setThrottleConfig]);

  useEffect(() => {
    registerSave("throttle", handleSave);
    return () => unregisterSave("throttle");
  }, [registerSave, unregisterSave, handleSave]);

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
        <Switch
          checked={throttleEnabled}
          onCheckedChange={(v) => {
            setThrottleEnabled(v);
            markDirty("throttle");
          }}
        />
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
                if (v) {
                  setThrottlePreset(v);
                  markDirty("throttle");
                }
              }}
            >
              <SelectTrigger className="w-64">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THROTTLE_PRESETS.map((preset) => (
                  <SelectItem key={preset.value} value={preset.value} label={preset.label}>
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
                    onChange={(e) => {
                      setThrottleDownload(e.target.value);
                      markDirty("throttle");
                    }}
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
                    onChange={(e) => {
                      setThrottleUpload(e.target.value);
                      markDirty("throttle");
                    }}
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
                    onChange={(e) => {
                      setThrottleLatency(e.target.value);
                      markDirty("throttle");
                    }}
                  />
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
