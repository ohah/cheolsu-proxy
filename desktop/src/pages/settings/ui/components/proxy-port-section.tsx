import { Trans } from "@lingui/react/macro";
import { Input } from "@/shared/ui";
import { useSettingsForm } from "../settings-form";

export function ProxyPortSection() {
  const {
    register,
    watch,
    formState: { errors },
  } = useSettingsForm();
  const port = watch("proxyPort");

  return (
    <div className="border rounded-lg p-5 space-y-5">
      <div>
        <h2 className="text-lg font-semibold">
          <Trans>Proxy Port</Trans>
        </h2>
        <p className="text-sm text-muted-foreground">
          <Trans>
            Port number for the proxy server to listen on. Changes take effect on next app restart.
          </Trans>
        </p>
      </div>
      <div className="w-32">
        <Input
          type="number"
          min={1}
          max={65535}
          {...register("proxyPort", {
            valueAsNumber: true,
            validate: (v) =>
              (Number.isInteger(v) && v >= 1 && v <= 65535) || "Port must be between 1 and 65535",
          })}
        />
        {errors.proxyPort && (
          <p className="text-xs text-red-600 mt-1">
            <Trans>Port must be between 1 and 65535</Trans>
          </p>
        )}
      </div>
      {!errors.proxyPort && port !== 8100 && (
        <p className="text-xs text-amber-600 dark:text-amber-400">
          <Trans>
            Default port is 8100. Make sure your system proxy is configured to use port {port}.
          </Trans>
        </p>
      )}
    </div>
  );
}
