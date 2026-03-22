import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useHostMappingStore, useHostMappingDialogStore } from "@/shared/stores";
import { Badge, Button, Switch, RuleListPage } from "@/shared/ui";
import { Trash2, ArrowRight } from "lucide-react";
import { toast } from "sonner";
import type { HostMapping } from "@/shared/api/proxy";
import { HostMappingFormDialog } from "@/features/host-mapping-form";

export const HostMappingPage = () => {
  const { t } = useLingui();
  const { hostMappings, removeMapping, toggleMapping, clearMappings } = useHostMappingStore();
  const dialogStore = useHostMappingDialogStore();

  const handleDelete = (id: string) => {
    removeMapping(id);
    toast.success(t`Host mapping deleted`);
  };

  const handleClearAll = () => {
    clearMappings();
    toast.success(t`All host mappings cleared`);
  };

  const formatMapping = (mapping: HostMapping) => {
    const src = mapping.source_port
      ? `${mapping.source_host}:${mapping.source_port}`
      : mapping.source_host;
    const tgt = mapping.target_port
      ? `${mapping.target_host}:${mapping.target_port}`
      : mapping.target_host;
    return { src, tgt };
  };

  return (
    <RuleListPage<HostMapping>
      title={<Trans>Host Mapping</Trans>}
      description={
        <Trans>Map DNS hostnames to different target hosts for testing and development</Trans>
      }
      badgeLabel={<Trans>mappings</Trans>}
      emptyTitle={<Trans>No host mappings</Trans>}
      emptyDescription={
        <Trans>Add mappings to redirect DNS hostnames to different target hosts for testing.</Trans>
      }
      emptyAddLabel={<Trans>Add your first mapping</Trans>}
      addLabel={<Trans>Add Mapping</Trans>}
      items={hostMappings}
      getItemKey={(mapping) => mapping.id}
      onAdd={() => dialogStore.openEmpty()}
      onClearAll={handleClearAll}
      renderItem={(mapping) => {
        const { src, tgt } = formatMapping(mapping);
        return (
          <div className="flex items-center gap-4">
            <Switch checked={mapping.enabled} onCheckedChange={() => toggleMapping(mapping.id)} />

            <div
              className={`flex-1 min-w-0 transition-opacity ${!mapping.enabled ? "opacity-50" : ""}`}
            >
              <div className="flex items-center gap-2 mb-1">
                <code className="text-sm font-medium bg-muted px-2 py-0.5 rounded truncate">
                  {src}
                </code>
                <ArrowRight className="w-4 h-4 text-muted-foreground shrink-0" />
                <code className="text-sm font-medium bg-muted px-2 py-0.5 rounded truncate">
                  {tgt}
                </code>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={mapping.enabled ? "default" : "secondary"} className="text-xs">
                  {mapping.enabled ? <Trans>Enabled</Trans> : <Trans>Disabled</Trans>}
                </Badge>
                {mapping.source_host.includes("*") && (
                  <Badge variant="outline" className="text-xs">
                    <Trans>Wildcard</Trans>
                  </Badge>
                )}
              </div>
            </div>

            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleDelete(mapping.id)}
                title={t`Delete mapping`}
                className="text-destructive hover:text-destructive"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            </div>
          </div>
        );
      }}
      dialogs={
        <HostMappingFormDialog
          open={dialogStore.open}
          onOpenChange={(open) => !open && dialogStore.close()}
          initialValues={dialogStore.initialValues}
        />
      }
    />
  );
};
