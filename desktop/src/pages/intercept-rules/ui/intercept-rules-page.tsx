import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useInterceptRuleStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Plus, Trash2, Pencil, Ban, ArrowUpDown, ArrowDownUp, Eraser, Replace } from "lucide-react";
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
  }
}

const ACTION_LABELS: Record<
  string,
  { labelKey: string; variant: "default" | "secondary" | "destructive" | "outline" }
> = {
  block: { labelKey: "block", variant: "destructive" },
  modify_request: { labelKey: "modify_request", variant: "default" },
  modify_response: { labelKey: "modify_response", variant: "secondary" },
  map_local: { labelKey: "map_local", variant: "outline" },
  map_remote: { labelKey: "map_remote", variant: "outline" },
  rewrite: { labelKey: "rewrite", variant: "default" },
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
    <>
      <div className="flex-1 flex flex-col h-full overflow-auto">
        <div className="p-6 space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">
                <Trans>Intercept Rules</Trans>
              </h1>
              <p className="text-muted-foreground">
                <Trans>Manage wildcard-based request/response intercept rules</Trans>
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="text-sm">
                {rules.length} <Trans>rules</Trans>
              </Badge>
              {rules.length > 0 && (
                <Button variant="outline" size="sm" onClick={handleClearAll}>
                  <Eraser className="w-4 h-4 mr-1" />
                  <Trans>Clear All</Trans>
                </Button>
              )}
              <Button size="sm" onClick={handleAdd}>
                <Plus className="w-4 h-4 mr-1" />
                <Trans>Add Rule</Trans>
              </Button>
            </div>
          </div>

          {rules.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <div className="text-center space-y-2">
                  <h3 className="text-lg font-semibold">
                    <Trans>No intercept rules</Trans>
                  </h3>
                  <p className="text-muted-foreground">
                    <Trans>
                      Add rules to intercept, block, or modify HTTP requests and responses.
                    </Trans>
                  </p>
                  <Button className="mt-4" onClick={handleAdd}>
                    <Plus className="w-4 h-4 mr-1" />
                    <Trans>Add your first rule</Trans>
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {rules.map((rule) => {
                const actionInfo = ACTION_LABELS[rule.action.type];
                return (
                  <Card key={rule.id}>
                    <CardContent className="py-4">
                      <div className="flex items-center gap-4">
                        <Switch
                          checked={rule.enabled}
                          onCheckedChange={() => toggleRule(rule.id)}
                        />

                        <div
                          className={`flex-1 min-w-0 transition-opacity ${!rule.enabled ? "opacity-50" : ""}`}
                        >
                          <div className="flex items-center gap-2 mb-1">
                            <span className="font-medium text-sm truncate">
                              {rule.name || rule.pattern}
                            </span>
                            {rule.name && (
                              <code className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded truncate max-w-[300px]">
                                {rule.pattern}
                              </code>
                            )}
                          </div>
                          <div className="flex items-center gap-2">
                            <Badge variant={actionInfo.variant} className="text-xs gap-1">
                              {getActionIcon(rule.action.type)}
                              {actionLabelMap[actionInfo.labelKey]}
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
                                  +{Object.keys(rule.action.add_headers).length}{" "}
                                  <Trans>headers</Trans>
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
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <RuleFormDialog open={dialogOpen} onOpenChange={setDialogOpen} editingRule={editingRule} />
    </>
  );
};
