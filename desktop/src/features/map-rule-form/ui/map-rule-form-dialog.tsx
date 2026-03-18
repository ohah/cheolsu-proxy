import { useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useForm, FormProvider, Controller } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  Button,
  Input,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/shared/ui";
import { toast } from "sonner";
import { useMapRuleStore } from "@/shared/stores";
import type { InterceptRule, MapLocalAction, MapRemoteAction } from "@/entities/intercept-rule";
import {
  mapRuleFormSchema,
  defaultMapRuleFormValues,
  type MapRuleFormValues,
} from "@/entities/intercept-rule";
import { HTTP_METHODS } from "@/shared/lib/http-constants";
import { MapLocalFields } from "./map-local-fields";
import { MapRemoteFields } from "./map-remote-fields";

type MapType = "map_local" | "map_remote";

interface MapRuleFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingRule: InterceptRule | null;
}

const editingRuleToFormValues = (rule: InterceptRule): MapRuleFormValues => {
  const base = {
    name: rule.name,
    pattern: rule.pattern,
    method: rule.method ?? "*",
  };

  if (rule.action.type === "map_local") {
    const action = rule.action as MapLocalAction;
    return {
      ...base,
      action: {
        type: "map_local",
        file_path: action.file_path,
        status_code: String(action.status_code),
        headers: Object.entries(action.headers).map(([key, value]) => ({
          key,
          value,
        })),
      },
    };
  } else if (rule.action.type === "map_remote") {
    const action = rule.action as MapRemoteAction;
    return {
      ...base,
      action: {
        type: "map_remote",
        target_url: action.target_url,
        preserve_path: action.preserve_path,
      },
    };
  }

  return { ...base, action: defaultMapRuleFormValues.action };
};

export const MapRuleFormDialog = ({
  open: isOpen,
  onOpenChange,
  editingRule,
}: MapRuleFormDialogProps) => {
  const { t } = useLingui();
  const { addRule, updateRule } = useMapRuleStore();

  const form = useForm<MapRuleFormValues>({
    resolver: zodResolver(mapRuleFormSchema),
    defaultValues: defaultMapRuleFormValues,
  });

  const mapType = form.watch("action.type") as MapType;

  useEffect(() => {
    if (!isOpen) return;

    if (editingRule) {
      form.reset(editingRuleToFormValues(editingRule));
    } else {
      form.reset(defaultMapRuleFormValues);
    }
  }, [isOpen, editingRule, form]);

  const handleMapTypeChange = (newType: MapType) => {
    if (newType === "map_local") {
      form.setValue("action", {
        type: "map_local",
        file_path: "",
        status_code: "200",
        headers: [],
      });
    } else {
      form.setValue("action", {
        type: "map_remote",
        target_url: "",
        preserve_path: true,
      });
    }
  };

  const onSubmit = (values: MapRuleFormValues) => {
    const { action: formAction } = values;

    const action =
      formAction.type === "map_local"
        ? {
            type: "map_local" as const,
            file_path: formAction.file_path.trim(),
            status_code: parseInt(formAction.status_code) || 200,
            headers: Object.fromEntries(
              formAction.headers.filter((h) => h.key.trim()).map((h) => [h.key.trim(), h.value]),
            ),
          }
        : {
            type: "map_remote" as const,
            target_url: formAction.target_url.trim(),
            preserve_path: formAction.preserve_path,
          };

    const rule: InterceptRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: values.name.trim() || values.pattern.trim(),
      enabled: editingRule?.enabled ?? true,
      pattern: values.pattern.trim(),
      method: values.method === "*" ? null : values.method,
      action,
    };

    if (editingRule) {
      updateRule(rule);
      toast.success(t`Rule updated`);
    } else {
      addRule(rule);
      toast.success(t`Rule added`);
    }

    onOpenChange(false);
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[560px] max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {editingRule ? <Trans>Edit Map Rule</Trans> : <Trans>Add Map Rule</Trans>}
          </DialogTitle>
          <DialogDescription>
            <Trans>
              Map Local: respond with a local file. Map Remote: redirect to another URL.
            </Trans>
          </DialogDescription>
        </DialogHeader>

        <FormProvider {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {/* Name */}
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Name</Trans>
              </label>
              <Input placeholder={t`Rule name (optional)`} {...form.register("name")} />
            </div>

            {/* Pattern */}
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>URL Pattern</Trans> <span className="text-destructive">*</span>
              </label>
              <Input
                placeholder="*.example.com/api/* or https://api.example.com/data"
                {...form.register("pattern")}
              />
              <p className="text-xs text-muted-foreground">
                <Trans>* = any string, ? = single character</Trans>
              </p>
              {form.formState.errors.pattern && (
                <p className="text-destructive text-xs">{form.formState.errors.pattern.message}</p>
              )}
            </div>

            {/* Method & Map Type */}
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Method</Trans>
                </label>
                <Controller
                  control={form.control}
                  name="method"
                  render={({ field }) => (
                    <Select value={field.value} onValueChange={(v) => v && field.onChange(v)}>
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="*" label={t`All Methods`}>{t`All Methods`}</SelectItem>
                        {HTTP_METHODS.map((m) => (
                          <SelectItem key={m} value={m}>
                            {m}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Type</Trans>
                </label>
                <Select
                  value={mapType}
                  onValueChange={(v) => v && handleMapTypeChange(v as MapType)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="map_local" label={t`Map Local`}>{t`Map Local`}</SelectItem>
                    <SelectItem
                      value="map_remote"
                      label={t`Map Remote`}
                    >{t`Map Remote`}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>

            {/* Type-specific fields */}
            {mapType === "map_local" && <MapLocalFields />}
            {mapType === "map_remote" && <MapRemoteFields />}

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                <Trans>Cancel</Trans>
              </Button>
              <Button type="submit">
                {editingRule ? <Trans>Update</Trans> : <Trans>Add Rule</Trans>}
              </Button>
            </DialogFooter>
          </form>
        </FormProvider>
      </DialogContent>
    </Dialog>
  );
};
