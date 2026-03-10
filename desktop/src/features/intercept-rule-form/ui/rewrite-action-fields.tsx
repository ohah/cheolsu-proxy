import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext, Controller } from "react-hook-form";
import {
  Input,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/shared/ui";
import type { InterceptRuleFormValues } from "@/entities/intercept-rule";

export const RewriteActionFields = () => {
  const { t } = useLingui();
  const { register, control } = useFormContext<InterceptRuleFormValues>();

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Target</Trans>
        </label>
        <Controller
          control={control}
          name={"action.rewrite_target" as never}
          render={({ field }) => (
            <Select
              value={field.value as string}
              onValueChange={(v) => v && field.onChange(v)}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="request_header">{t`Request Header`}</SelectItem>
                <SelectItem value="response_header">{t`Response Header`}</SelectItem>
                <SelectItem value="request_body">{t`Request Body`}</SelectItem>
                <SelectItem value="response_body">{t`Response Body`}</SelectItem>
              </SelectContent>
            </Select>
          )}
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Match Pattern</Trans> <span className="text-destructive">*</span>
        </label>
        <Input
          placeholder={t`Regex pattern (e.g. old-domain\\.com)`}
          {...register("action.match_pattern" as never)}
          className="font-mono text-xs"
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Replace With</Trans>
        </label>
        <Input
          placeholder={t`Replacement string (supports $1, $2 capture groups)`}
          {...register("action.replace_with" as never)}
          className="font-mono text-xs"
        />
      </div>
    </>
  );
};
