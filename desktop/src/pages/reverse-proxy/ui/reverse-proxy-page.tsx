import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useReverseProxyStore } from "@/shared/stores/reverse-proxy-store";
import { Card, CardContent, Badge, Button, Switch, Input, RuleListPage } from "@/shared/ui";
import { Trash2, ArrowRight } from "lucide-react";
import { toast } from "sonner";
import type { ReverseProxyRule } from "@/shared/api/proxy";

function generateId(): string {
  return `rp_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

export const ReverseProxyPage = () => {
  const { t } = useLingui();
  const { rules, addRule, removeRule, toggleRule, clearRules } = useReverseProxyStore();
  const [showForm, setShowForm] = useState(false);
  const [matchHost, setMatchHost] = useState("");
  const [backendScheme, setBackendScheme] = useState("http");
  const [backendHost, setBackendHost] = useState("");
  const [backendPort, setBackendPort] = useState("");
  const [rewriteHost, setRewriteHost] = useState(true);

  const isValidPort = (port: string): boolean => {
    if (!port) return false;
    const num = parseInt(port, 10);
    return Number.isInteger(num) && num >= 1 && num <= 65535;
  };

  const handleAdd = () => {
    if (!matchHost.trim() || !backendHost.trim()) {
      toast.error(t`Match host and backend host are required`);
      return;
    }

    if (!isValidPort(backendPort)) {
      toast.error(t`Backend port must be between 1 and 65535`);
      return;
    }

    const rule: ReverseProxyRule = {
      id: generateId(),
      match_host: matchHost.trim(),
      backend_scheme: backendScheme,
      backend_host: backendHost.trim(),
      backend_port: parseInt(backendPort, 10),
      rewrite_host: rewriteHost,
      enabled: true,
    };

    addRule(rule);
    toast.success(t`Reverse proxy rule added`);
    setMatchHost("");
    setBackendScheme("http");
    setBackendHost("");
    setBackendPort("");
    setRewriteHost(true);
    setShowForm(false);
  };

  const handleDelete = (id: string) => {
    removeRule(id);
    toast.success(t`Reverse proxy rule deleted`);
  };

  const handleClearAll = () => {
    clearRules();
    toast.success(t`All reverse proxy rules cleared`);
  };

  const inlineForm = showForm ? (
    <Card>
      <CardContent className="py-4">
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Match Host</Trans>
              </label>
              <Input
                placeholder="api.myapp.local"
                value={matchHost}
                onChange={(e) => setMatchHost(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Backend Scheme</Trans>
              </label>
              <div className="flex gap-2">
                <Button
                  variant={backendScheme === "http" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setBackendScheme("http")}
                >
                  HTTP
                </Button>
                <Button
                  variant={backendScheme === "https" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setBackendScheme("https")}
                >
                  HTTPS
                </Button>
              </div>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Backend Host</Trans>
              </label>
              <Input
                placeholder="httpbin.org"
                value={backendHost}
                onChange={(e) => setBackendHost(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Backend Port</Trans>
              </label>
              <Input
                placeholder="80"
                type="number"
                value={backendPort}
                onChange={(e) => setBackendPort(e.target.value)}
              />
            </div>
          </div>
          <div className="flex items-center justify-between">
            <label className="flex items-center gap-2 text-sm">
              <Switch checked={rewriteHost} onCheckedChange={setRewriteHost} />
              <Trans>Rewrite Host header to backend address</Trans>
            </label>
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
  ) : null;

  return (
    <RuleListPage<ReverseProxyRule>
      title={<Trans>Reverse Proxy</Trans>}
      description={
        <Trans>Route incoming requests to backend servers based on Host header matching</Trans>
      }
      badgeLabel={<Trans>rules</Trans>}
      emptyTitle={<Trans>No reverse proxy rules</Trans>}
      emptyDescription={
        <Trans>
          Add rules to route requests with relative URIs to backend servers based on Host header.
        </Trans>
      }
      emptyAddLabel={<Trans>Add your first rule</Trans>}
      addLabel={<Trans>Add Rule</Trans>}
      items={rules}
      getItemKey={(rule) => rule.id}
      onAdd={() => setShowForm(!showForm)}
      onClearAll={handleClearAll}
      headerExtra={inlineForm}
      renderItem={(rule) => {
        const backend = `${rule.backend_scheme}://${rule.backend_host}:${rule.backend_port}`;
        return (
          <div className="flex items-center gap-4">
            <Switch checked={rule.enabled} onCheckedChange={() => toggleRule(rule.id)} />

            <div
              className={`flex-1 min-w-0 transition-opacity ${!rule.enabled ? "opacity-50" : ""}`}
            >
              <div className="flex items-center gap-2 mb-1">
                <code className="text-sm font-medium bg-muted px-2 py-0.5 rounded truncate">
                  {rule.match_host}
                </code>
                <ArrowRight className="w-4 h-4 text-muted-foreground shrink-0" />
                <code className="text-sm font-medium bg-muted px-2 py-0.5 rounded truncate">
                  {backend}
                </code>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant={rule.enabled ? "default" : "secondary"} className="text-xs">
                  {rule.enabled ? <Trans>Enabled</Trans> : <Trans>Disabled</Trans>}
                </Badge>
                {rule.match_host.includes("*") && (
                  <Badge variant="outline" className="text-xs">
                    <Trans>Wildcard</Trans>
                  </Badge>
                )}
                {rule.rewrite_host && (
                  <Badge variant="outline" className="text-xs">
                    <Trans>Rewrite Host</Trans>
                  </Badge>
                )}
              </div>
            </div>

            <div className="flex items-center gap-1">
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
    />
  );
};
