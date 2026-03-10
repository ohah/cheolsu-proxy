import { useState, useEffect } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
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
  Textarea,
} from "@/shared/ui";
import { Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { useInterceptRuleStore } from "@/shared/stores";
import {
  useHeaderEditor,
  headersToEntries,
  entriesToHeaders,
} from "@/shared/hooks/use-header-editor";
import type {
  InterceptRule,
  InterceptAction,
  InterceptActionType,
  RewriteTarget,
} from "@/entities/intercept-rule";
import type { InterceptRuleInitialValues } from "@/shared/stores";

interface RuleFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingRule: InterceptRule | null;
  initialValues?: InterceptRuleInitialValues | null;
}

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export const RuleFormDialog = ({
  open,
  onOpenChange,
  editingRule,
  initialValues,
}: RuleFormDialogProps) => {
  const { t } = useLingui();
  const { addRule, updateRule } = useInterceptRuleStore();

  const [name, setName] = useState("");
  const [pattern, setPattern] = useState("");
  const [method, setMethod] = useState<string>("*");
  const [actionType, setActionType] = useState<InterceptActionType>("block");
  const [statusCode, setStatusCode] = useState("403");
  const [body, setBody] = useState("");
  const [responseStatus, setResponseStatus] = useState("");
  const { headers, addHeader, removeHeader, updateHeader, resetHeaders } = useHeaderEditor();
  const [removeHeaders, setRemoveHeaders] = useState<string[]>([]);
  const [rewriteTarget, setRewriteTarget] = useState<RewriteTarget>("request_header");
  const [matchPattern, setMatchPattern] = useState("");
  const [replaceWith, setReplaceWith] = useState("");

  useEffect(() => {
    if (!open) return;

    if (editingRule) {
      setName(editingRule.name);
      setPattern(editingRule.pattern);
      setMethod(editingRule.method ?? "*");
      setActionType(editingRule.action.type);

      if (editingRule.action.type === "block") {
        setStatusCode(String(editingRule.action.status_code));
        setBody(editingRule.action.body);
        resetHeaders([]);
        setRemoveHeaders([]);
        setResponseStatus("");
      } else if (editingRule.action.type === "modify_request") {
        setBody(editingRule.action.set_body ?? "");
        resetHeaders(headersToEntries(editingRule.action.add_headers));
        setRemoveHeaders(editingRule.action.remove_headers);
        setStatusCode("403");
        setResponseStatus("");
      } else if (editingRule.action.type === "modify_response") {
        setResponseStatus(
          editingRule.action.set_status ? String(editingRule.action.set_status) : "",
        );
        setBody(editingRule.action.set_body ?? "");
        resetHeaders(headersToEntries(editingRule.action.add_headers));
        setRemoveHeaders(editingRule.action.remove_headers);
        setStatusCode("403");
      } else if (editingRule.action.type === "rewrite") {
        setRewriteTarget(editingRule.action.target);
        setMatchPattern(editingRule.action.match_pattern);
        setReplaceWith(editingRule.action.replace_with);
        setStatusCode("403");
        setBody("");
        setResponseStatus("");
        resetHeaders([]);
        setRemoveHeaders([]);
      }
    } else if (initialValues) {
      setName("");
      setPattern(initialValues.pattern);
      setMethod(initialValues.method ?? "*");
      setActionType("block");
      setStatusCode("403");
      setBody("");
      setResponseStatus("");
      resetHeaders([]);
      setRemoveHeaders([]);
      setRewriteTarget("request_header");
      setMatchPattern("");
      setReplaceWith("");
    } else {
      setName("");
      setPattern("");
      setMethod("*");
      setActionType("block");
      setStatusCode("403");
      setBody("");
      setResponseStatus("");
      resetHeaders([]);
      setRemoveHeaders([]);
      setRewriteTarget("request_header");
      setMatchPattern("");
      setReplaceWith("");
    }
  }, [open, editingRule, initialValues]);

  const buildAction = (): InterceptAction => {
    switch (actionType) {
      case "block":
        return {
          type: "block",
          status_code: parseInt(statusCode) || 403,
          body,
        };
      case "modify_request":
        return {
          type: "modify_request",
          add_headers: entriesToHeaders(headers),
          remove_headers: removeHeaders.filter((h) => h.trim()),
          set_body: body.trim() || null,
        };
      case "modify_response":
        return {
          type: "modify_response",
          set_status: responseStatus ? parseInt(responseStatus) || null : null,
          add_headers: entriesToHeaders(headers),
          remove_headers: removeHeaders.filter((h) => h.trim()),
          set_body: body.trim() || null,
        };
      case "rewrite":
        return {
          type: "rewrite",
          target: rewriteTarget,
          match_pattern: matchPattern,
          replace_with: replaceWith,
        };
      default:
        return {
          type: "block",
          status_code: 403,
          body: "",
        };
    }
  };

  const handleSubmit = () => {
    if (!pattern.trim()) {
      toast.error(t`Pattern is required`);
      return;
    }

    if (actionType === "rewrite" && !matchPattern.trim()) {
      toast.error(t`Match pattern is required`);
      return;
    }

    const rule: InterceptRule = {
      id: editingRule?.id ?? crypto.randomUUID(),
      name: name.trim() || pattern.trim(),
      enabled: editingRule?.enabled ?? true,
      pattern: pattern.trim(),
      method: method === "*" ? null : method,
      action: buildAction(),
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

  const addRemoveHeader = () => setRemoveHeaders([...removeHeaders, ""]);
  const deleteRemoveHeader = (index: number) =>
    setRemoveHeaders(removeHeaders.filter((_, i) => i !== index));
  const updateRemoveHeader = (index: number, value: string) => {
    const updated = [...removeHeaders];
    updated[index] = value;
    setRemoveHeaders(updated);
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

        <div className="space-y-4">
          {/* Name */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              <Trans>Name</Trans>
            </label>
            <Input
              placeholder={t`Rule name (optional)`}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          {/* Pattern */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              <Trans>Pattern</Trans> <span className="text-destructive">*</span>
            </label>
            <Input
              placeholder="*.example.com/api/*"
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
            />
          </div>

          {/* Method & Action Type */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Method</Trans>
              </label>
              <Select value={method} onValueChange={(v) => v && setMethod(v)}>
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
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Action</Trans>
              </label>
              <Select
                value={actionType}
                onValueChange={(v) => v && setActionType(v as InterceptActionType)}
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
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Block-specific: status code */}
          {actionType === "block" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Status Code</Trans>
              </label>
              <Input
                type="number"
                placeholder="403"
                value={statusCode}
                onChange={(e) => setStatusCode(e.target.value)}
              />
            </div>
          )}

          {/* ModifyResponse: status code */}
          {actionType === "modify_response" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Response Status Code</Trans>
              </label>
              <Input
                type="number"
                placeholder={t`200 (optional)`}
                value={responseStatus}
                onChange={(e) => setResponseStatus(e.target.value)}
              />
            </div>
          )}

          {/* Headers (for modify_request / modify_response) */}
          {(actionType === "modify_request" || actionType === "modify_response") && (
            <>
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="text-sm font-medium">
                    <Trans>Add Headers</Trans>
                  </label>
                  <Button variant="ghost" size="sm" onClick={addHeader}>
                    <Plus className="w-3.5 h-3.5 mr-1" />
                    <Trans>Add</Trans>
                  </Button>
                </div>
                {headers.map((header, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Input
                      placeholder={t`Header name`}
                      value={header.key}
                      onChange={(e) => updateHeader(i, "key", e.target.value)}
                      className="flex-1"
                    />
                    <Input
                      placeholder={t`Value`}
                      value={header.value}
                      onChange={(e) => updateHeader(i, "value", e.target.value)}
                      className="flex-1"
                    />
                    <Button variant="ghost" size="sm" onClick={() => removeHeader(i)}>
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
                  <Button variant="ghost" size="sm" onClick={addRemoveHeader}>
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
                    <Button variant="ghost" size="sm" onClick={() => deleteRemoveHeader(i)}>
                      <Trash2 className="w-3.5 h-3.5 text-destructive" />
                    </Button>
                  </div>
                ))}
              </div>
            </>
          )}

          {/* Rewrite fields */}
          {actionType === "rewrite" && (
            <>
              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Target</Trans>
                </label>
                <Select
                  value={rewriteTarget}
                  onValueChange={(v) => v && setRewriteTarget(v as RewriteTarget)}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      value="request_header"
                      label={t`Request Header`}
                    >{t`Request Header`}</SelectItem>
                    <SelectItem
                      value="response_header"
                      label={t`Response Header`}
                    >{t`Response Header`}</SelectItem>
                    <SelectItem
                      value="request_body"
                      label={t`Request Body`}
                    >{t`Request Body`}</SelectItem>
                    <SelectItem
                      value="response_body"
                      label={t`Response Body`}
                    >{t`Response Body`}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Match Pattern</Trans> <span className="text-destructive">*</span>
                </label>
                <Input
                  placeholder={t`Regex pattern (e.g. old-domain\\.com)`}
                  value={matchPattern}
                  onChange={(e) => setMatchPattern(e.target.value)}
                  className="font-mono text-xs"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">
                  <Trans>Replace With</Trans>
                </label>
                <Input
                  placeholder={t`Replacement string (supports $1, $2 capture groups)`}
                  value={replaceWith}
                  onChange={(e) => setReplaceWith(e.target.value)}
                  className="font-mono text-xs"
                />
              </div>
            </>
          )}

          {/* Body */}
          {actionType !== "rewrite" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium">
                <Trans>Body</Trans>
              </label>
              <Textarea
                placeholder={
                  actionType === "block" ? t`Response body (optional)` : t`Set body (optional)`
                }
                value={body}
                onChange={(e) => setBody(e.target.value)}
                rows={4}
                className="font-mono text-xs"
              />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            <Trans>Cancel</Trans>
          </Button>
          <Button onClick={handleSubmit}>
            {editingRule ? <Trans>Update</Trans> : <Trans>Add Rule</Trans>}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
