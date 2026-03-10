import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext, useFieldArray } from "react-hook-form";
import { Input, Button } from "@/shared/ui";
import { Plus, Trash2 } from "lucide-react";
import type { InterceptRuleFormValues } from "@/entities/intercept-rule";

export const HeadersFields = () => {
  const { t } = useLingui();
  const { register, control, watch, setValue } =
    useFormContext<InterceptRuleFormValues>();

  const {
    fields: headerFields,
    append: appendHeader,
    remove: removeHeaderAt,
  } = useFieldArray({
    control,
    name: "action.headers" as never,
  });

  const removeHeaders: string[] =
    (watch("action.remove_headers" as never) as string[] | undefined) ?? [];

  const addRemoveHeader = () => {
    setValue("action.remove_headers" as never, [
      ...removeHeaders,
      "",
    ] as never);
  };

  const deleteRemoveHeader = (index: number) => {
    setValue(
      "action.remove_headers" as never,
      removeHeaders.filter((_, i) => i !== index) as never,
    );
  };

  const updateRemoveHeader = (index: number, value: string) => {
    const updated = [...removeHeaders];
    updated[index] = value;
    setValue("action.remove_headers" as never, updated as never);
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
            onClick={() => appendHeader({ key: "", value: "" } as never)}
          >
            <Plus className="w-3.5 h-3.5 mr-1" />
            <Trans>Add</Trans>
          </Button>
        </div>
        {headerFields.map((field, i) => (
          <div key={field.id} className="flex items-center gap-2">
            <Input
              placeholder={t`Header name`}
              {...register(`action.headers.${i}.key` as never)}
              className="flex-1"
            />
            <Input
              placeholder={t`Value`}
              {...register(`action.headers.${i}.value` as never)}
              className="flex-1"
            />
            <Button
              variant="ghost"
              size="sm"
              type="button"
              onClick={() => removeHeaderAt(i)}
            >
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
          <Button
            variant="ghost"
            size="sm"
            type="button"
            onClick={addRemoveHeader}
          >
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
            <Button
              variant="ghost"
              size="sm"
              type="button"
              onClick={() => deleteRemoveHeader(i)}
            >
              <Trash2 className="w-3.5 h-3.5 text-destructive" />
            </Button>
          </div>
        ))}
      </div>
    </>
  );
};
