import { Trans } from "@lingui/react/macro";
import { useFormContext, Controller } from "react-hook-form";
import { Input, Switch } from "@/shared/ui";
import type { MapRemoteFormValues } from "@/entities/intercept-rule";

export const MapRemoteFields = () => {
  const { register, control } = useFormContext<MapRemoteFormValues>();

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Target URL</Trans> <span className="text-destructive">*</span>
        </label>
        <Input
          placeholder="http://localhost:3000 or https://staging.example.com"
          {...register("action.target_url")}
        />
      </div>

      <Controller
        control={control}
        name="action.preserve_path"
        render={({ field }) => (
          <div className="flex items-center gap-3">
            <Switch checked={field.value as boolean} onCheckedChange={field.onChange} />
            <div>
              <label className="text-sm font-medium">
                <Trans>Preserve Path</Trans>
              </label>
              <p className="text-xs text-muted-foreground">
                <Trans>Append the original request path to the target URL</Trans>
              </p>
            </div>
          </div>
        )}
      />
    </>
  );
};
