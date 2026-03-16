import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useContractStore } from "@/shared/stores/contract-store";
import { useTransactionStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Trash2, Plus, FileCheck, AlertTriangle, XCircle } from "lucide-react";
import { toast } from "sonner";
import { open } from "@tauri-apps/plugin-dialog";

export const ContractTestingPage = () => {
  const { t } = useLingui();
  const { specs, loadSpec, unloadSpec, toggleSpec } = useContractStore();
  const violationTransactions = useTransactionStore((s) =>
    s.transactions.filter(
      (tx) => tx.validations && tx.validations.some((v) => v.violations.length > 0),
    ),
  );

  const handleAddSpec = async () => {
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "OpenAPI Spec",
          extensions: ["json", "yaml", "yml"],
        },
      ],
    });

    if (path) {
      try {
        await loadSpec(path as string);
        toast.success(t`OpenAPI spec loaded`);
      } catch (e) {
        toast.error(t`Failed to load spec: ${String(e)}`);
      }
    }
  };

  const handleRemoveSpec = async (id: string) => {
    try {
      await unloadSpec(id);
      toast.success(t`Spec removed`);
    } catch (e) {
      toast.error(t`Failed to remove spec: ${String(e)}`);
    }
  };

  const handleToggleSpec = async (id: string, enabled: boolean) => {
    try {
      await toggleSpec(id, enabled);
    } catch (e) {
      toast.error(t`Failed to toggle spec: ${String(e)}`);
    }
  };

  const totalErrors = violationTransactions.reduce(
    (sum, tx) =>
      sum +
      (tx.validations?.reduce(
        (s, v) => s + v.violations.filter((vl) => vl.severity === "Error").length,
        0,
      ) ?? 0),
    0,
  );

  const totalWarnings = violationTransactions.reduce(
    (sum, tx) =>
      sum +
      (tx.validations?.reduce(
        (s, v) => s + v.violations.filter((vl) => vl.severity === "Warning").length,
        0,
      ) ?? 0),
    0,
  );

  return (
    <div className="flex flex-col h-full p-4 gap-4 overflow-auto">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileCheck className="h-5 w-5" />
          <h1 className="text-lg font-semibold">
            <Trans>Contract Testing</Trans>
          </h1>
        </div>
        <Button size="sm" onClick={handleAddSpec}>
          <Plus className="h-4 w-4 mr-1" />
          <Trans>Add Spec</Trans>
        </Button>
      </div>

      {/* Spec 관리 */}
      <Card>
        <CardContent className="p-4">
          <h2 className="text-sm font-medium mb-3">
            <Trans>OpenAPI Specs</Trans>
          </h2>
          {specs.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-6">
              <Trans>
                No specs loaded. Add an OpenAPI/Swagger spec file to start contract testing.
              </Trans>
            </p>
          ) : (
            <div className="space-y-2">
              {specs.map((spec) => (
                <div
                  key={spec.id}
                  className="flex items-center justify-between p-3 border rounded-md"
                >
                  <div className="flex items-center gap-3 min-w-0 flex-1">
                    <Switch
                      checked={spec.enabled}
                      onCheckedChange={(checked) => handleToggleSpec(spec.id, checked)}
                    />
                    <div className="min-w-0">
                      <div className="text-sm font-medium truncate">{spec.name}</div>
                      <div className="text-xs text-muted-foreground truncate">{spec.file_path}</div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <Badge variant="secondary" className="text-xs">
                      {spec.path_count} <Trans>paths</Trans>
                    </Badge>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8"
                      onClick={() => handleRemoveSpec(spec.id)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* 검증 리포트 */}
      <Card>
        <CardContent className="p-4">
          <h2 className="text-sm font-medium mb-3">
            <Trans>Validation Report</Trans>
          </h2>

          <div className="flex items-center gap-6 mb-4">
            <div className="text-center">
              <div className="text-2xl font-bold">{violationTransactions.length}</div>
              <div className="text-xs text-muted-foreground">
                <Trans>Violations</Trans>
              </div>
            </div>
            {totalErrors > 0 && (
              <div className="flex items-center gap-1 text-destructive">
                <XCircle className="h-4 w-4" />
                <span className="text-sm font-medium">
                  {totalErrors} <Trans>errors</Trans>
                </span>
              </div>
            )}
            {totalWarnings > 0 && (
              <div className="flex items-center gap-1 text-yellow-500">
                <AlertTriangle className="h-4 w-4" />
                <span className="text-sm font-medium">
                  {totalWarnings} <Trans>warnings</Trans>
                </span>
              </div>
            )}
            {violationTransactions.length === 0 && specs.length > 0 && (
              <p className="text-sm text-muted-foreground">
                <Trans>All requests match the spec</Trans>
              </p>
            )}
          </div>

          {violationTransactions.length > 0 && (
            <div className="space-y-1 max-h-96 overflow-auto">
              {violationTransactions.map((tx) => {
                const violations = tx.validations?.flatMap((v) => v.violations) ?? [];
                const errors = violations.filter((v) => v.severity === "Error").length;
                const warnings = violations.filter((v) => v.severity === "Warning").length;

                return (
                  <div
                    key={tx.request?.id}
                    className="flex items-center gap-3 p-2 border rounded-md text-sm"
                  >
                    <Badge variant="outline" className="text-xs shrink-0">
                      {tx.request?.method}
                    </Badge>
                    <span className="truncate flex-1 font-mono text-xs">{tx.request?.uri}</span>
                    <div className="flex items-center gap-2 shrink-0">
                      {errors > 0 && <span className="text-destructive text-xs">{errors}E</span>}
                      {warnings > 0 && <span className="text-yellow-500 text-xs">{warnings}W</span>}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
};
