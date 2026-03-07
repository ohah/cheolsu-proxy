import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useMapRuleStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Plus, Trash2, Pencil, FileDown, GitBranch, Eraser } from "lucide-react";
import { toast } from "sonner";
import type { InterceptRule } from "@/entities/intercept-rule";
import { MapRuleFormDialog } from "@/features/map-rule-form";

function getMapIcon(type: string) {
  switch (type) {
    case "map_local":
      return <FileDown className="w-3.5 h-3.5" />;
    case "map_remote":
      return <GitBranch className="w-3.5 h-3.5" />;
    default:
      return null;
  }
}

const MAP_LABELS: Record<string, { labelKey: string; variant: "default" | "secondary" }> = {
  map_local: { labelKey: "map_local", variant: "default" },
  map_remote: { labelKey: "map_remote", variant: "secondary" },
};

export const MapRulesPage = () => {
  const { t } = useLingui();
  const { rules, removeRule, toggleRule, clearRules } = useMapRuleStore();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<InterceptRule | null>(null);

  const mapLabelMap: Record<string, string> = {
    map_local: t`Map Local`,
    map_remote: t`Map Remote`,
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
                <Trans>Map Rules</Trans>
              </h1>
              <p className="text-muted-foreground">
                <Trans>
                  Map Local: respond with local files. Map Remote: redirect requests to another URL.
                </Trans>
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
                    <Trans>No map rules</Trans>
                  </h3>
                  <p className="text-muted-foreground">
                    <Trans>
                      Add rules to map URLs to local files or redirect to other servers.
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
                const mapInfo = MAP_LABELS[rule.action.type] ?? {
                  labelKey: rule.action.type,
                  variant: "default" as const,
                };
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
                            <Badge variant={mapInfo.variant} className="text-xs gap-1">
                              {getMapIcon(rule.action.type)}
                              {mapLabelMap[mapInfo.labelKey] ?? mapInfo.labelKey}
                            </Badge>
                            {rule.method && (
                              <Badge variant="outline" className="text-xs">
                                {rule.method}
                              </Badge>
                            )}
                            {rule.action.type === "map_local" && (
                              <span className="text-xs text-muted-foreground truncate max-w-[300px]">
                                {(rule.action as { file_path: string }).file_path}
                              </span>
                            )}
                            {rule.action.type === "map_remote" && (
                              <span className="text-xs text-muted-foreground truncate max-w-[300px]">
                                → {(rule.action as { target_url: string }).target_url}
                                {(rule.action as { preserve_path: boolean }).preserve_path &&
                                  t` (preserve path)`}
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

      <MapRuleFormDialog open={dialogOpen} onOpenChange={setDialogOpen} editingRule={editingRule} />
    </>
  );
};
