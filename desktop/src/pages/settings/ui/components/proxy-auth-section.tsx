import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Input, Switch } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function ProxyAuthSection() {
  const { t } = useLingui();
  const { register, watch, setValue } = useSettingsForm();
  const enabled = watch("proxyAuth.enabled");

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">
            <Trans>Proxy Authentication</Trans>
          </h2>
          <p className="text-sm text-muted-foreground">
            <Trans>Require authentication to use this proxy server</Trans>
          </p>
        </div>
        <Switch
          checked={enabled}
          onCheckedChange={(v) => setValue("proxyAuth.enabled", v, { shouldDirty: true })}
        />
      </div>
      {enabled && (
        <div className="space-y-4 pt-2">
          <div className="flex gap-3">
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Username</Trans>
              </label>
              <Input placeholder={t`Username`} {...register("proxyAuth.username")} />
            </div>
            <div className="flex-1">
              <label className="text-sm font-medium mb-1.5 block">
                <Trans>Password</Trans>
              </label>
              <Input
                type="password"
                placeholder={t`Password`}
                {...register("proxyAuth.password")}
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            <Trans>
              Clients must provide these credentials via Proxy-Authorization header (HTTP Basic) to
              use this proxy
            </Trans>
          </p>
        </div>
      )}
    </div>
  );
}
