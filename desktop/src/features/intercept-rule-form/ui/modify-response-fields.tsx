import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext } from "react-hook-form";
import { Input, Textarea } from "@/shared/ui";
import type { InterceptRuleFormValues } from "@/entities/intercept-rule";
import { HeadersFields } from "./headers-fields";

export const ModifyResponseFields = () => {
  const { t } = useLingui();
  const { register } = useFormContext<InterceptRuleFormValues>();

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Response Status Code</Trans>
        </label>
        <Input
          type="number"
          placeholder={t`200 (optional)`}
          {...register("action.response_status" as never)}
        />
      </div>

      <HeadersFields />

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Body</Trans>
        </label>
        <Textarea
          placeholder={t`Set body (optional)`}
          {...register("action.body" as never)}
          rows={4}
          className="font-mono text-xs"
        />
      </div>
    </>
  );
};
