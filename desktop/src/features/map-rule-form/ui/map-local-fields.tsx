import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useFormContext, useFieldArray } from "react-hook-form";
import { Input, Button } from "@/shared/ui";
import { Plus, Trash2, FolderOpen } from "lucide-react";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import type { MapRuleFormValues } from "@/entities/intercept-rule";

export const MapLocalFields = () => {
  const { t } = useLingui();
  const { register, control, setValue } =
    useFormContext<MapRuleFormValues>();

  const {
    fields: headerFields,
    append: appendHeader,
    remove: removeHeader,
  } = useFieldArray({
    control,
    name: "action.headers" as never,
  });

  const handleSelectFile = async () => {
    try {
      const selected = await openFileDialog({
        multiple: false,
        title: t`Select local file`,
      });
      if (selected) {
        setValue("action.file_path" as never, selected as never);
      }
    } catch {
      // user cancelled
    }
  };

  return (
    <>
      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Local File Path</Trans> <span className="text-destructive">*</span>
        </label>
        <div className="flex gap-2">
          <Input
            placeholder="/path/to/response.json"
            {...register("action.file_path" as never)}
            className="flex-1"
          />
          <Button
            variant="outline"
            size="sm"
            type="button"
            onClick={handleSelectFile}
          >
            <FolderOpen className="w-4 h-4" />
          </Button>
        </div>
      </div>

      <div className="space-y-1.5">
        <label className="text-sm font-medium">
          <Trans>Status Code</Trans>
        </label>
        <Input
          type="number"
          placeholder="200"
          {...register("action.status_code" as never)}
        />
      </div>

      <div className="space-y-1.5">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            <Trans>Response Headers</Trans>
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
              onClick={() => removeHeader(i)}
            >
              <Trash2 className="w-3.5 h-3.5 text-destructive" />
            </Button>
          </div>
        ))}
      </div>
    </>
  );
};
