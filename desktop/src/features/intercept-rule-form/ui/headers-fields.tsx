import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext, useFieldArray } from "react-hook-form";
import { Input, Button } from "@/shared/ui";
import { Plus, Trash2 } from "lucide-react";
import type { ModifyRequestFormValues } from "@/entities/intercept-rule";

/**
 * HeadersFields는 modify_request / modify_response 액션에서 사용됩니다.
 * 두 액션 모두 action.headers, action.remove_headers 필드를 공유하므로
 * ModifyRequestFormValues 타입을 대표로 사용합니다.
 */
export const HeadersFields = () => {
  const { t } = useLingui();
  const { register, control, watch, setValue } = useFormContext<ModifyRequestFormValues>();

  const {
    fields: headerFields,
    append: appendHeader,
    remove: removeHeaderAt,
  } = useFieldArray({
    control,
    name: "action.headers",
  });

  const removeHeaders: string[] = watch("action.remove_headers") ?? [];

  const addRemoveHeader = () => {
    setValue("action.remove_headers", [...removeHeaders, ""]);
  };

  const deleteRemoveHeader = (index: number) => {
    setValue(
      "action.remove_headers",
      removeHeaders.filter((_, i) => i !== index),
    );
  };

  const updateRemoveHeader = (index: number, value: string) => {
    const updated = [...removeHeaders];
    updated[index] = value;
    setValue("action.remove_headers", updated);
  };

  return (
    <>
      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            <Trans>Add Headers</Trans>
          </label>
          <Button
            variant="ghost"
            size="sm"
            type="button"
            onClick={() => appendHeader({ key: "", value: "" })}
          >
            <Plus className="w-3.5 h-3.5 mr-1" />
            <Trans>Add</Trans>
          </Button>
        </div>
        {headerFields.map((field, i) => (
          <div key={field.id} className="flex items-center gap-2">
            <Input
              placeholder={t`Header name`}
              {...register(`action.headers.${i}.key`)}
              className="flex-1"
            />
            <Input
              placeholder={t`Value`}
              {...register(`action.headers.${i}.value`)}
              className="flex-1"
            />
            <Button variant="ghost" size="sm" type="button" onClick={() => removeHeaderAt(i)}>
              <Trash2 className="w-3.5 h-3.5 text-destructive" />
            </Button>
          </div>
        ))}
      </div>

      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            <Trans>Remove Headers</Trans>
          </label>
          <Button variant="ghost" size="sm" type="button" onClick={addRemoveHeader}>
            <Plus className="w-3.5 h-3.5 mr-1" />
            <Trans>Add</Trans>
          </Button>
        </div>
        {removeHeaders.map((header, i) => (
          <div key={i} className="flex items-center gap-2">
            <Input
              placeholder={t`Header name to remove`}
              value={header}
              onChange={(e) => updateRemoveHeader(i, e.target.value)}
              className="flex-1"
            />
            <Button variant="ghost" size="sm" type="button" onClick={() => deleteRemoveHeader(i)}>
              <Trash2 className="w-3.5 h-3.5 text-destructive" />
            </Button>
          </div>
        ))}
      </div>
    </>
  );
};
