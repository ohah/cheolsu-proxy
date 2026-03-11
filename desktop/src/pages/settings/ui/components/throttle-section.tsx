import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Input, Switch } from "@/shared/ui";
import { Select, SelectTrigger, SelectContent, SelectItem, SelectValue } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";
import { THROTTLE_PRESETS } from "../constants";

export function ThrottleSection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("throttle.enabled");
  const preset = watch("throttle.preset");

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
          checked={enabled}
          onCheckedChange={(v) => setValue("throttle.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div>
            <label className="text-sm font-medium mb-1.5 block">
              <Trans>Profile</Trans>
            </label>
            <Select
              value={preset}
              onValueChange={(v) => {
                if (v) setValue("throttle.preset", v, { shouldDirty: true });
              }}
            >
              <SelectTrigger className="w-64">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {THROTTLE_PRESETS.map((p) => (
                  <SelectItem key={p.value} value={p.value} label={p.label}>
                    {p.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          {preset === "custom" && (
            <div className="flex gap-3">
              <div className="flex-1">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Download (KB/s)</Trans>
                </label>
                <Input
                  type="number"
                  placeholder={t`Unlimited`}
                  {...register("throttle.download")}
                />
              </div>
              <div className="flex-1">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Upload (KB/s)</Trans>
                </label>
                <Input type="number" placeholder={t`Unlimited`} {...register("throttle.upload")} />
              </div>
              <div className="w-28">
                <label className="text-sm font-medium mb-1.5 block">
                  <Trans>Latency (ms)</Trans>
                </label>
                <Input type="number" placeholder="0" {...register("throttle.latency")} />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
