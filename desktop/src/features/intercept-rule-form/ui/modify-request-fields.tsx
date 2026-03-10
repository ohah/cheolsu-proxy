import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext } from "react-hook-form";
import { Textarea } from "@/shared/ui";
import type { ModifyRequestFormValues } from "@/entities/intercept-rule";
import { HeadersFields } from "./headers-fields";

export const ModifyRequestFields = () => {
  const { t } = useLingui();
  const { register } = useFormContext<ModifyRequestFormValues>();

  return (
    <>
      <HeadersFields />

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Body</Trans>
        </label>
        <Textarea
          placeholder={t`Set body (optional)`}
          {...register("action.body")}
          rows={4}
          className="font-mono text-xs"
        />
      </div>
    </>
  );
};
