import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext, Controller } from "react-hook-form";
import { Input, Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/shared/ui";
import type { ThrottleFormValues } from "@/entities/intercept-rule";

const THROTTLE_PRESETS: Record<string, { dl: string; ul: string; lat: string }> = {
  gprs: { dl: "50", ul: "20", lat: "500" },
  slow3g: { dl: "500", ul: "500", lat: "400" },
  fast3g: { dl: "1600", ul: "768", lat: "150" },
  lte: { dl: "4096", ul: "3072", lat: "50" },
  wifi: { dl: "30720", ul: "15360", lat: "2" },
};

export const ThrottleActionFields = () => {
  const { t } = useLingui();
  const { register, setValue, control } = useFormContext<ThrottleFormValues>();

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Latency (ms)</Trans>
        </label>
        <Input type="number" placeholder="0" {...register("action.latency_ms")} />
        <p className="text-xs text-muted-foreground">
          <Trans>Delay before forwarding the request (milliseconds)</Trans>
        </p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <label className="text-sm font-medium">
            <Trans>Download Speed (KB/s)</Trans>
          </label>
          <Input type="number" placeholder={t`Unlimited`} {...register("action.download_speed")} />
        </div>
        <div className="space-y-1.5">
          <label className="text-sm font-medium">
            <Trans>Upload Speed (KB/s)</Trans>
          </label>
          <Input type="number" placeholder={t`Unlimited`} {...register("action.upload_speed")} />
        </div>
      </div>

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Preset</Trans>
        </label>
        <Controller
          control={control}
          name="action.latency_ms"
          render={() => (
            <Select
              value="custom"
              onValueChange={(v) => {
                if (!v || v === "custom") return;
                const p = THROTTLE_PRESETS[v];
                if (p) {
                  setValue("action.download_speed", p.dl);
                  setValue("action.upload_speed", p.ul);
                  setValue("action.latency_ms", p.lat);
                }
              }}
            >
              <SelectTrigger className="w-full">
                <SelectValue placeholder={t`Select preset...`} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="custom" label={t`Custom`}>{t`Custom`}</SelectItem>
                <SelectItem value="gprs" label="GPRS (50KB/s)">
                  GPRS (50KB/s, 500ms)
                </SelectItem>
                <SelectItem value="slow3g" label="Slow 3G (500KB/s)">
                  Slow 3G (500KB/s, 400ms)
                </SelectItem>
                <SelectItem value="fast3g" label="Fast 3G (1.6MB/s)">
                  Fast 3G (1.6MB/s, 150ms)
                </SelectItem>
                <SelectItem value="lte" label="LTE (4MB/s)">
                  LTE (4MB/s, 50ms)
                </SelectItem>
                <SelectItem value="wifi" label="WiFi (30MB/s)">
                  WiFi (30MB/s, 2ms)
                </SelectItem>
              </SelectContent>
            </Select>
          )}
        />
      </div>
    </>
  );
};
