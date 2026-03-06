import { useState } from "react";
import { useProxyStore, useMapRuleStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Plus, Trash2, Pencil, FileDown, GitBranch, Eraser } from "lucide-react";
import { toast } from "sonner";
import { AppSidebar } from "@/shared/app-sidebar";
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

const MAP_LABELS: Record<string, { label: string; variant: "default" | "secondary" }> = {
  map_local: { label: "Map Local", variant: "default" },
  map_remote: { label: "Map Remote", variant: "secondary" },
};

export const MapRulesPage = () => {
  const { isConnected } = useProxyStore();
  const { rules, removeRule, toggleRule, clearRules } = useMapRuleStore();
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
              <h1 className="text-2xl font-bold text-foreground">Map Rules</h1>
              <p className="text-muted-foreground">
                Map Local: respond with local files. Map Remote: redirect requests to another URL.
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
                  <h3 className="text-lg font-semibold">No map rules</h3>
                  <p className="text-muted-foreground">
                    Add rules to map URLs to local files or redirect to other servers.
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
                const mapInfo = MAP_LABELS[rule.action.type] ?? {
                  label: rule.action.type,
                  variant: "default" as const,
                };
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
                            <Badge variant={mapInfo.variant} className="text-xs gap-1">
                              {getMapIcon(rule.action.type)}
                              {mapInfo.label}
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
                                  " (preserve path)"}
                              </span>
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

      <MapRuleFormDialog open={dialogOpen} onOpenChange={setDialogOpen} editingRule={editingRule} />
    </div>
  );
};
