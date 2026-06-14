import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useInterceptRuleStore } from "@/shared/stores";
import { Badge, Button, Switch, RuleListPage } from "@/shared/ui";
import { Trash2, Pencil, Ban, ArrowUpDown, ArrowDownUp, Replace, Gauge } from "lucide-react";
import { toast } from "sonner";
import type { InterceptRule } from "@/entities/intercept-rule";
import { RuleFormDialog } from "@/features/intercept-rule-form";

function getActionIcon(type: string) {
  switch (type) {
    case "block":
      return <Ban className="w-3.5 h-3.5" />;
    case "modify_request":
      return <ArrowUpDown className="w-3.5 h-3.5" />;
    case "modify_response":
      return <ArrowDownUp className="w-3.5 h-3.5" />;
    case "rewrite":
      return <Replace className="w-3.5 h-3.5" />;
    case "throttle":
      return <Gauge className="w-3.5 h-3.5" />;
  }
}

const ACTION_LABELS: Record<
  string,
  {
    labelKey: string;
    variant: "default" | "secondary" | "destructive" | "outline";
  }
> = {
  block: { labelKey: "block", variant: "destructive" },
  modify_request: { labelKey: "modify_request", variant: "default" },
  modify_response: { labelKey: "modify_response", variant: "secondary" },
  map_local: { labelKey: "map_local", variant: "outline" },
  map_remote: { labelKey: "map_remote", variant: "outline" },
  rewrite: { labelKey: "rewrite", variant: "default" },
  throttle: { labelKey: "throttle", variant: "secondary" },
};

export const InterceptRulesPage = () => {
  const { t } = useLingui();
  const { rules, removeRule, toggleRule, clearRules } = useInterceptRuleStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<InterceptRule | null>(null);

  const actionLabelMap: Record<string, string> = {
    block: t`Block`,
    modify_request: t`Modify Request`,
    modify_response: t`Modify Response`,
    map_local: t`Map Local`,
    map_remote: t`Map Remote`,
    rewrite: t`Rewrite`,
    throttle: t`Throttle`,
  };

  const handleAdd = () => {
    setEditingRule(null);
    setDialogOpen(true);
  };

  const handleEdit = (rule: InterceptRule) => {
    setEditingRule(rule);
    setDialogOpen(true);
  };

  const handleDelete = (id: string) => {
    removeRule(id);
    toast.success(t`Rule deleted`);
  };

  const handleClearAll = () => {
    clearRules();
    toast.success(t`All rules cleared`);
  };

  return (
    <RuleListPage<InterceptRule>
      title={<Trans>Intercept Rules</Trans>}
      description={<Trans>Manage wildcard-based request/response intercept rules</Trans>}
      badgeLabel={<Trans>rules</Trans>}
      emptyTitle={<Trans>No intercept rules</Trans>}
      emptyDescription={
        <Trans>Add rules to intercept, block, or modify HTTP requests and responses.</Trans>
      }
      emptyAddLabel={<Trans>Add your first rule</Trans>}
      addLabel={<Trans>Add Rule</Trans>}
      items={rules}
      getItemKey={(rule) => rule.id}
      onAdd={handleAdd}
      onClearAll={handleClearAll}
      renderItem={(rule) => {
        // 알 수 없는 action type에도 크래시하지 않도록 fallback 제공
        const actionInfo = ACTION_LABELS[rule.action.type] ?? {
          labelKey: rule.action.type,
          variant: "outline" as const,
        };
        return (
          <div className="flex items-center gap-4">
            <Switch checked={rule.enabled} onCheckedChange={() => toggleRule(rule.id)} />

            <div
              className={`flex-1 min-w-0 transition-opacity ${!rule.enabled ? "opacity-50" : ""}`}
            >
              <div className="flex items-center gap-2 mb-1">
                <span className="font-medium text-sm truncate">{rule.name || rule.pattern}</span>
                {rule.name && (
                  <code className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded truncate max-w-[300px]">
                    {rule.pattern}
                  </code>
                )}
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={actionInfo.variant} className="text-xs gap-1">
                  {getActionIcon(rule.action.type)}
                  {actionLabelMap[actionInfo.labelKey] ?? actionInfo.labelKey}
                </Badge>
                {rule.method && (
                  <Badge variant="outline" className="text-xs">
                    {rule.method}
                  </Badge>
                )}
                {rule.action.type === "block" && (
                  <span className="text-xs text-muted-foreground">
                    <Trans>Status</Trans>: {rule.action.status_code}
                  </span>
                )}
                {rule.action.type === "modify_response" && rule.action.set_status && (
                  <span className="text-xs text-muted-foreground">
                    <Trans>Status</Trans>: {rule.action.set_status}
                  </span>
                )}
                {(rule.action.type === "modify_request" ||
                  rule.action.type === "modify_response") &&
                  Object.keys(rule.action.add_headers).length > 0 && (
                    <span className="text-xs text-muted-foreground">
                      +{Object.keys(rule.action.add_headers).length} <Trans>headers</Trans>
                    </span>
                  )}
                {(rule.action.type === "modify_request" ||
                  rule.action.type === "modify_response") &&
                  rule.action.set_body && (
                    <span className="text-xs text-muted-foreground">
                      <Trans>Custom body</Trans>
                    </span>
                  )}
                {rule.action.type === "rewrite" && (
                  <span className="text-xs text-muted-foreground font-mono">
                    s/{rule.action.match_pattern}/{rule.action.replace_with}/
                  </span>
                )}
              </div>
            </div>

            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleEdit(rule)}
                title={t`Edit rule`}
              >
                <Pencil className="w-4 h-4" />
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleDelete(rule.id)}
                title={t`Delete rule`}
                className="text-destructive hover:text-destructive"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            </div>
          </div>
        );
      }}
      dialogs={
        <RuleFormDialog open={dialogOpen} onOpenChange={setDialogOpen} editingRule={editingRule} />
      }
    />
  );
};
