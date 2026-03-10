import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext } from "react-hook-form";
import { Input, Textarea } from "@/shared/ui";
import type { InterceptRuleFormValues } from "@/entities/intercept-rule";

export const BlockActionFields = () => {
  const { t } = useLingui();
  const { register } = useFormContext<InterceptRuleFormValues>();

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Status Code</Trans>
        </label>
        <Input type="number" placeholder="403" {...register("action.status_code")} />
      </div>

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Body</Trans>
        </label>
        <Textarea
          placeholder={t`Response body (optional)`}
          {...register("action.body")}
          rows={4}
          className="font-mono text-xs"
        />
      </div>
    </>
  );
};
