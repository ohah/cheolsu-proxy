import { useState, useEffect } from "react";
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
import type { InterceptRule, InterceptAction, InterceptActionType } from "@/entities/intercept-rule";

interface RuleFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editingRule: InterceptRule | null;
}

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export const RuleFormDialog = ({
  open,
  onOpenChange,
  editingRule,
}: RuleFormDialogProps) => {
  const { addRule, updateRule } = useInterceptRuleStore();

  const [name, setName] = useState("");
  const [pattern, setPattern] = useState("");
  const [method, setMethod] = useState<string>("*");
  const [actionType, setActionType] = useState<InterceptActionType>("block");
  const [statusCode, setStatusCode] = useState("403");
  const [body, setBody] = useState("");
  const [responseStatus, setResponseStatus] = useState("");
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([]);
  const [removeHeaders, setRemoveHeaders] = useState<string[]>([]);

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
        setHeaders([]);
        setRemoveHeaders([]);
        setResponseStatus("");
      } else if (editingRule.action.type === "modify_request") {
        setBody(editingRule.action.set_body ?? "");
        setHeaders(
          Object.entries(editingRule.action.add_headers).map(([key, value]) => ({ key, value })),
        );
        setRemoveHeaders(editingRule.action.remove_headers);
        setStatusCode("403");
        setResponseStatus("");
      } else if (editingRule.action.type === "modify_response") {
        setResponseStatus(editingRule.action.set_status ? String(editingRule.action.set_status) : "");
        setBody(editingRule.action.set_body ?? "");
        setHeaders(
          Object.entries(editingRule.action.add_headers).map(([key, value]) => ({ key, value })),
        );
        setRemoveHeaders(editingRule.action.remove_headers);
        setStatusCode("403");
      }
    } else {
      setName("");
      setPattern("");
      setMethod("*");
      setActionType("block");
      setStatusCode("403");
      setBody("");
      setResponseStatus("");
      setHeaders([]);
      setRemoveHeaders([]);
    }
  }, [open, editingRule]);

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
          add_headers: Object.fromEntries(
            headers.filter((h) => h.key.trim()).map((h) => [h.key.trim(), h.value]),
          ),
          remove_headers: removeHeaders.filter((h) => h.trim()),
          set_body: body.trim() || null,
        };
      case "modify_response":
        return {
          type: "modify_response",
          set_status: responseStatus ? parseInt(responseStatus) || null : null,
          add_headers: Object.fromEntries(
            headers.filter((h) => h.key.trim()).map((h) => [h.key.trim(), h.value]),
          ),
          remove_headers: removeHeaders.filter((h) => h.trim()),
          set_body: body.trim() || null,
        };
    }
  };

  const handleSubmit = () => {
    if (!pattern.trim()) {
      toast.error("Pattern is required");
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
      toast.success("Rule updated");
    } else {
      addRule(rule);
      toast.success("Rule added");
    }

    onOpenChange(false);
  };

  const addHeader = () => setHeaders([...headers, { key: "", value: "" }]);
  const removeHeader = (index: number) => setHeaders(headers.filter((_, i) => i !== index));
  const updateHeader = (index: number, field: "key" | "value", value: string) => {
    const updated = [...headers];
    updated[index] = { ...updated[index], [field]: value };
    setHeaders(updated);
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
          <DialogTitle>{editingRule ? "Edit Rule" : "Add Rule"}</DialogTitle>
          <DialogDescription>
            Use wildcard patterns: * matches any string, ? matches a single character.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Name */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Name</label>
            <Input
              placeholder="Rule name (optional)"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          {/* Pattern */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">
              Pattern <span className="text-destructive">*</span>
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
              <label className="text-sm font-medium">Method</label>
              <Select value={method} onValueChange={(v) => v && setMethod(v)}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="*">All Methods</SelectItem>
                  {HTTP_METHODS.map((m) => (
                    <SelectItem key={m} value={m}>
                      {m}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">Action</label>
              <Select
                value={actionType}
                onValueChange={(v) => v && setActionType(v as InterceptActionType)}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="block">Block</SelectItem>
                  <SelectItem value="modify_request">Modify Request</SelectItem>
                  <SelectItem value="modify_response">Modify Response</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Block-specific: status code */}
          {actionType === "block" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium">Status Code</label>
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
              <label className="text-sm font-medium">Response Status Code</label>
              <Input
                type="number"
                placeholder="200 (optional)"
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
                  <label className="text-sm font-medium">Add Headers</label>
                  <Button variant="ghost" size="sm" onClick={addHeader}>
                    <Plus className="w-3.5 h-3.5 mr-1" />
                    Add
                  </Button>
                </div>
                {headers.map((header, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Input
                      placeholder="Header name"
                      value={header.key}
                      onChange={(e) => updateHeader(i, "key", e.target.value)}
                      className="flex-1"
                    />
                    <Input
                      placeholder="Value"
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
                  <label className="text-sm font-medium">Remove Headers</label>
                  <Button variant="ghost" size="sm" onClick={addRemoveHeader}>
                    <Plus className="w-3.5 h-3.5 mr-1" />
                    Add
                  </Button>
                </div>
                {removeHeaders.map((header, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <Input
                      placeholder="Header name to remove"
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

          {/* Body */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Body</label>
            <Textarea
              placeholder={actionType === "block" ? "Response body (optional)" : "Set body (optional)"}
              value={body}
              onChange={(e) => setBody(e.target.value)}
              rows={4}
              className="font-mono text-xs"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit}>
            {editingRule ? "Update" : "Add Rule"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
