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
import { useInterceptRuleStore } from "@/shared/stores";
import { interceptRuleFormSchema, defaultInterceptRuleFormValues } from "@/entities/intercept-rule";
import type {
  InterceptRule,
  InterceptActionType,
  InterceptRuleFormValues,
} from "@/entities/intercept-rule";
import type { InterceptRuleInitialValues } from "@/shared/stores";
import { HTTP_METHODS } from "@/shared/lib/http-constants";
import {
  ruleToFormValues,
  formValuesToAction,
  initialValuesToFormValues,
} from "../lib/form-converters";
import { BlockActionFields } from "./block-action-fields";
import { ModifyRequestFields } from "./modify-request-fields";
import { ModifyResponseFields } from "./modify-response-fields";
import { RewriteActionFields } from "./rewrite-action-fields";
import { ThrottleActionFields } from "./throttle-action-fields";

interface RuleFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingRule: InterceptRule | null;
  initialValues?: InterceptRuleInitialValues | null;
}

const ACTION_TYPE_DEFAULTS: Record<InterceptActionType, InterceptRuleFormValues["action"]> = {
  block: { type: "block", status_code: "403", body: "" },
  modify_request: { type: "modify_request", headers: [], remove_headers: [], body: "" },
  modify_response: {
    type: "modify_response",
    response_status: "",
    headers: [],
    remove_headers: [],
    body: "",
  },
  rewrite: {
    type: "rewrite",
    rewrite_target: "request_header",
    match_pattern: "",
    replace_with: "",
  },
  throttle: { type: "throttle", latency_ms: "0", download_speed: "", upload_speed: "" },
  map_local: { type: "block", status_code: "403", body: "" },
  map_remote: { type: "block", status_code: "403", body: "" },
};

export const RuleFormDialog = ({
  open,
  onOpenChange,
  editingRule,
  initialValues,
}: RuleFormDialogProps) => {
  const { t } = useLingui();
  const { addRule, updateRule } = useInterceptRuleStore();

  const methods = useForm<InterceptRuleFormValues>({
    resolver: zodResolver(interceptRuleFormSchema),
    defaultValues: defaultInterceptRuleFormValues,
  });

  const { reset, handleSubmit, watch, setValue, register, control } = methods;
  const actionType = watch("action.type");

  useEffect(() => {
    if (!open) return;

    if (editingRule) {
      reset(ruleToFormValues(editingRule));
    } else if (initialValues) {
      reset(initialValuesToFormValues(initialValues));
    } else {
      reset(defaultInterceptRuleFormValues);
    }
  }, [open, editingRule, initialValues, reset]);

  const onActionTypeChange = (newType: InterceptActionType) => {
    const defaults = ACTION_TYPE_DEFAULTS[newType];
    if (defaults) {
      setValue("action", defaults);
    }
  };

  const onSubmit = (values: InterceptRuleFormValues) => {
    const rule: InterceptRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: values.name.trim() || values.pattern.trim(),
      enabled: editingRule?.enabled ?? true,
      pattern: values.pattern.trim(),
      method: values.method === "*" ? null : values.method,
      action: formValuesToAction(values),
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

  const onError = () => {
    toast.error(t`Please fill in all required fields`);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[560px] max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {editingRule ? <Trans>Edit Rule</Trans> : <Trans>Add Rule</Trans>}
          </DialogTitle>
          <DialogDescription>
            <Trans>
              Use wildcard patterns: * matches any string, ? matches a single character.
            </Trans>
          </DialogDescription>
        </DialogHeader>

        <FormProvider {...methods}>
          <form onSubmit={handleSubmit(onSubmit, onError)} className="space-y-4">
            {/* Name */}
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Name</Trans>
              </label>
              <Input placeholder={t`Rule name (optional)`} {...register("name")} />
            </div>

            {/* Pattern */}
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Pattern</Trans> <span className="text-destructive">*</span>
              </label>
              <Input placeholder="*.example.com/api/*" {...register("pattern")} />
            </div>

            {/* Method & Action Type */}
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Method</Trans>
                </label>
                <Controller
                  control={control}
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
                  <Trans>Action</Trans>
                </label>
                <Controller
                  control={control}
                  name="action.type"
                  render={({ field }) => (
                    <Select
                      value={field.value}
                      onValueChange={(v) => {
                        if (v) {
                          onActionTypeChange(v as InterceptActionType);
                        }
                      }}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="block" label={t`Block`}>{t`Block`}</SelectItem>
                        <SelectItem
                          value="modify_request"
                          label={t`Modify Request`}
                        >{t`Modify Request`}</SelectItem>
                        <SelectItem
                          value="modify_response"
                          label={t`Modify Response`}
                        >{t`Modify Response`}</SelectItem>
                        <SelectItem value="rewrite" label={t`Rewrite`}>{t`Rewrite`}</SelectItem>
                        <SelectItem value="throttle" label={t`Throttle`}>{t`Throttle`}</SelectItem>
                      </SelectContent>
                    </Select>
                  )}
                />
              </div>
            </div>

            {/* Action-specific fields */}
            {actionType === "block" && <BlockActionFields />}
            {actionType === "modify_request" && <ModifyRequestFields />}
            {actionType === "modify_response" && <ModifyResponseFields />}
            {actionType === "rewrite" && <RewriteActionFields />}
            {actionType === "throttle" && <ThrottleActionFields />}

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
