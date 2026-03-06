import { useState } from "react";
import { useProxyStore, useInterceptRuleStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Plus, Trash2, Pencil, Ban, ArrowUpDown, ArrowDownUp, Eraser } from "lucide-react";
import { toast } from "sonner";
import { AppSidebar } from "@/shared/app-sidebar";
import type { InterceptRule, InterceptActionType } from "@/entities/intercept-rule";
import { RuleFormDialog } from "./rule-form-dialog";

const ACTION_LABELS: Record<
  InterceptActionType,
  { label: string; variant: "default" | "secondary" | "destructive" | "outline" }
> = {
  block: { label: "Block", variant: "destructive" },
  modify_request: { label: "Modify Request", variant: "default" },
  modify_response: { label: "Modify Response", variant: "secondary" },
};

function getActionIcon(type: InterceptActionType) {
  switch (type) {
    case "block":
      return <Ban className="w-3.5 h-3.5" />;
    case "modify_request":
      return <ArrowUpDown className="w-3.5 h-3.5" />;
    case "modify_response":
      return <ArrowDownUp className="w-3.5 h-3.5" />;
  }
}

export const InterceptRulesPage = () => {
  const { isConnected } = useProxyStore();
  const { rules, removeRule, toggleRule, clearRules } = useInterceptRuleStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<InterceptRule | null>(null);

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
    toast.success("Rule deleted");
  };

  const handleClearAll = () => {
    clearRules();
    toast.success("All rules cleared");
  };

  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar isConnected={isConnected} />

      <div className="flex-1 flex flex-col h-full overflow-auto">
        <div className="p-6 space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">Intercept Rules</h1>
              <p className="text-muted-foreground">
                Manage wildcard-based request/response intercept rules
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="text-sm">
                {rules.length} rules
              </Badge>
              {rules.length > 0 && (
                <Button variant="outline" size="sm" onClick={handleClearAll}>
                  <Eraser className="w-4 h-4 mr-1" />
                  Clear All
                </Button>
              )}
              <Button size="sm" onClick={handleAdd}>
                <Plus className="w-4 h-4 mr-1" />
                Add Rule
              </Button>
            </div>
          </div>

          {rules.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <div className="text-center space-y-2">
                  <h3 className="text-lg font-semibold">No intercept rules</h3>
                  <p className="text-muted-foreground">
                    Add rules to intercept, block, or modify HTTP requests and responses.
                  </p>
                  <Button className="mt-4" onClick={handleAdd}>
                    <Plus className="w-4 h-4 mr-1" />
                    Add your first rule
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {rules.map((rule) => {
                const actionInfo = ACTION_LABELS[rule.action.type];
                return (
                  <Card
                    key={rule.id}
                    className={`transition-opacity ${!rule.enabled ? "opacity-50" : ""}`}
                  >
                    <CardContent className="py-4">
                      <div className="flex items-center gap-4">
                        <Switch
                          checked={rule.enabled}
                          onCheckedChange={() => toggleRule(rule.id)}
                        />

                        <div className="flex-1 min-w-0">
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
                              {actionInfo.label}
                            </Badge>
                            {rule.method && (
                              <Badge variant="outline" className="text-xs">
                                {rule.method}
                              </Badge>
                            )}
                            {rule.action.type === "block" && (
                              <span className="text-xs text-muted-foreground">
                                Status: {rule.action.status_code}
                              </span>
                            )}
                            {rule.action.type === "modify_response" && rule.action.set_status && (
                              <span className="text-xs text-muted-foreground">
                                Status: {rule.action.set_status}
                              </span>
                            )}
                            {(rule.action.type === "modify_request" ||
                              rule.action.type === "modify_response") &&
                              Object.keys(rule.action.add_headers).length > 0 && (
                                <span className="text-xs text-muted-foreground">
                                  +{Object.keys(rule.action.add_headers).length} headers
                                </span>
                              )}
                            {(rule.action.type === "modify_request" ||
                              rule.action.type === "modify_response") &&
                              rule.action.set_body && (
                                <span className="text-xs text-muted-foreground">Custom body</span>
                              )}
                          </div>
                        </div>

                        <div className="flex items-center gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleEdit(rule)}
                            title="Edit rule"
                          >
                            <Pencil className="w-4 h-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleDelete(rule.id)}
                            title="Delete rule"
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
    </div>
  );
};
