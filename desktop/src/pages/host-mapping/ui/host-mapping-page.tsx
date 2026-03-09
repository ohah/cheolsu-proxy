import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useHostMappingStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch, Input } from "@/shared/ui";
import { Plus, Trash2, Eraser, ArrowRight } from "lucide-react";
import { toast } from "sonner";
import type { HostMapping } from "@/shared/api/proxy";

function generateId(): string {
  return `hm_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

export const HostMappingPage = () => {
  const { t } = useLingui();
  const { hostMappings, addMapping, removeMapping, toggleMapping, clearMappings } =
    useHostMappingStore();
  const [showForm, setShowForm] = useState(false);
  const [sourceHost, setSourceHost] = useState("");
  const [sourcePort, setSourcePort] = useState("");
  const [targetHost, setTargetHost] = useState("");
  const [targetPort, setTargetPort] = useState("");

  const isValidPort = (port: string): boolean => {
    if (!port) return true; // optional
    const num = parseInt(port, 10);
    return Number.isInteger(num) && num >= 1 && num <= 65535;
  };

  const handleAdd = () => {
    if (!sourceHost.trim() || !targetHost.trim()) {
      toast.error(t`Source host and target host are required`);
      return;
    }

    if (!isValidPort(sourcePort) || !isValidPort(targetPort)) {
      toast.error(t`Port must be between 1 and 65535`);
      return;
    }

    const mapping: HostMapping = {
      id: generateId(),
      source_host: sourceHost.trim(),
      source_port: sourcePort ? parseInt(sourcePort, 10) : null,
      target_host: targetHost.trim(),
      target_port: targetPort ? parseInt(targetPort, 10) : null,
      enabled: true,
    };

    addMapping(mapping);
    toast.success(t`Host mapping added`);
    setSourceHost("");
    setSourcePort("");
    setTargetHost("");
    setTargetPort("");
    setShowForm(false);
  };

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
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Host Mapping</Trans>
            </h1>
            <p className="text-muted-foreground">
              <Trans>Map DNS hostnames to different target hosts for testing and development</Trans>
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="text-sm">
              {hostMappings.length} <Trans>mappings</Trans>
            </Badge>
            {hostMappings.length > 0 && (
              <Button variant="outline" size="sm" onClick={handleClearAll}>
                <Eraser className="w-4 h-4 mr-1" />
                <Trans>Clear All</Trans>
              </Button>
            )}
            <Button size="sm" onClick={() => setShowForm(!showForm)}>
              <Plus className="w-4 h-4 mr-1" />
              <Trans>Add Mapping</Trans>
            </Button>
          </div>
        </div>

        {showForm && (
          <Card>
            <CardContent className="py-4">
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      <Trans>Source Host</Trans>
                    </label>
                    <Input
                      placeholder="*.api.example.com"
                      value={sourceHost}
                      onChange={(e) => setSourceHost(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      <Trans>Source Port</Trans>{" "}
                      <span className="text-muted-foreground text-xs">
                        (<Trans>optional</Trans>)
                      </span>
                    </label>
                    <Input
                      placeholder="443"
                      type="number"
                      value={sourcePort}
                      onChange={(e) => setSourcePort(e.target.value)}
                    />
                  </div>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      <Trans>Target Host</Trans>
                    </label>
                    <Input
                      placeholder="192.168.1.100"
                      value={targetHost}
                      onChange={(e) => setTargetHost(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      <Trans>Target Port</Trans>{" "}
                      <span className="text-muted-foreground text-xs">
                        (<Trans>optional</Trans>)
                      </span>
                    </label>
                    <Input
                      placeholder="8443"
                      type="number"
                      value={targetPort}
                      onChange={(e) => setTargetPort(e.target.value)}
                    />
                  </div>
                </div>
                <div className="flex justify-end gap-2">
                  <Button variant="outline" size="sm" onClick={() => setShowForm(false)}>
                    <Trans>Cancel</Trans>
                  </Button>
                  <Button size="sm" onClick={handleAdd}>
                    <Trans>Add</Trans>
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {hostMappings.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <div className="text-center space-y-2">
                <h3 className="text-lg font-semibold">
                  <Trans>No host mappings</Trans>
                </h3>
                <p className="text-muted-foreground">
                  <Trans>
                    Add mappings to redirect DNS hostnames to different target hosts for testing.
                  </Trans>
                </p>
                <Button className="mt-4" onClick={() => setShowForm(true)}>
                  <Plus className="w-4 h-4 mr-1" />
                  <Trans>Add your first mapping</Trans>
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {hostMappings.map((mapping) => {
              const { src, tgt } = formatMapping(mapping);
              return (
                <Card key={mapping.id}>
                  <CardContent className="py-4">
                    <div className="flex items-center gap-4">
                      <Switch
                        checked={mapping.enabled}
                        onCheckedChange={() => toggleMapping(mapping.id)}
                      />

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
                          <Badge
                            variant={mapping.enabled ? "default" : "secondary"}
                            className="text-xs"
                          >
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
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
