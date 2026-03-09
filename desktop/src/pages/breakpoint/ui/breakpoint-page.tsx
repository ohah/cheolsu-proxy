import { useState } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { useBreakpointStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { Plus, Trash2, Eraser, Play, XCircle, StopCircle } from "lucide-react";
import { toast } from "sonner";
import type { BreakpointRule, PendingBreakpoint } from "@/entities/breakpoint";

export const BreakpointPage = () => {
  const { t } = useLingui();
  const {
    rules,
    pendingBreakpoints,
    addRule,
    removeRule,
    toggleRule,
    clearRules,
    resolveBreakpoint,
  } = useBreakpointStore();

  const [showAddForm, setShowAddForm] = useState(false);
  const [newPattern, setNewPattern] = useState("");
  const [breakOnRequest, setBreakOnRequest] = useState(true);
  const [breakOnResponse, setBreakOnResponse] = useState(false);

  const handleAdd = () => {
    if (!newPattern.trim()) {
      toast.error(t`Pattern is required`);
      return;
    }
    const rule: BreakpointRule = {
      id: crypto.randomUUID(),
      pattern: newPattern.trim(),
      break_on_request: breakOnRequest,
      break_on_response: breakOnResponse,
      enabled: true,
    };
    addRule(rule);
    setNewPattern("");
    setBreakOnRequest(true);
    setBreakOnResponse(false);
    setShowAddForm(false);
    toast.success(t`Breakpoint rule added`);
  };

  const handleDelete = (id: string) => {
    removeRule(id);
    toast.success(t`Rule deleted`);
  };

  const handleClearAll = () => {
    clearRules();
    toast.success(t`All rules cleared`);
  };

  const handleForward = async (bp: PendingBreakpoint) => {
    try {
      await resolveBreakpoint(bp.id, { type: "forward" });
      toast.success(t`Breakpoint forwarded`);
    } catch {
      toast.error(t`Failed to forward breakpoint`);
    }
  };

  const handleDrop = async (bp: PendingBreakpoint) => {
    try {
      await resolveBreakpoint(bp.id, { type: "drop" });
      toast.success(t`Request dropped`);
    } catch {
      toast.error(t`Failed to drop request`);
    }
  };

  const handleAbort = async (bp: PendingBreakpoint) => {
    try {
      await resolveBreakpoint(bp.id, { type: "abort" });
      toast.success(t`Request aborted`);
    } catch {
      toast.error(t`Failed to abort request`);
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Breakpoints</Trans>
            </h1>
            <p className="text-muted-foreground">
              <Trans>Pause and inspect HTTP requests/responses in real-time</Trans>
            </p>
          </div>
          <div className="flex items-center gap-2">
            {pendingBreakpoints.length > 0 && (
              <Badge variant="destructive" className="text-sm">
                {pendingBreakpoints.length} <Trans>pending</Trans>
              </Badge>
            )}
            <Badge variant="outline" className="text-sm">
              {rules.length} <Trans>rules</Trans>
            </Badge>
            {rules.length > 0 && (
              <Button variant="outline" size="sm" onClick={handleClearAll}>
                <Eraser className="w-4 h-4 mr-1" />
                <Trans>Clear All</Trans>
              </Button>
            )}
            <Button size="sm" onClick={() => setShowAddForm(!showAddForm)}>
              <Plus className="w-4 h-4 mr-1" />
              <Trans>Add Rule</Trans>
            </Button>
          </div>
        </div>

        {/* Add Rule Form */}
        {showAddForm && (
          <Card>
            <CardContent className="py-4 space-y-3">
              <div className="flex items-center gap-3">
                <input
                  type="text"
                  className="flex-1 px-3 py-2 border rounded-md bg-background text-foreground text-sm"
                  placeholder={t`URL pattern (e.g., *api.example.com*)`}
                  value={newPattern}
                  onChange={(e) => setNewPattern(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleAdd()}
                  autoFocus
                />
                <Button size="sm" onClick={handleAdd}>
                  <Trans>Add</Trans>
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setShowAddForm(false)}>
                  <Trans>Cancel</Trans>
                </Button>
              </div>
              <div className="flex items-center gap-4 text-sm">
                <label className="flex items-center gap-2">
                  <Switch checked={breakOnRequest} onCheckedChange={setBreakOnRequest} />
                  <Trans>Break on Request</Trans>
                </label>
                <label className="flex items-center gap-2">
                  <Switch checked={breakOnResponse} onCheckedChange={setBreakOnResponse} />
                  <Trans>Break on Response</Trans>
                </label>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Pending Breakpoints */}
        {pendingBreakpoints.length > 0 && (
          <div className="space-y-3">
            <h2 className="text-lg font-semibold text-foreground">
              <Trans>Pending Breakpoints</Trans>
            </h2>
            {pendingBreakpoints.map((bp) => (
              <Card key={bp.id} className="border-orange-500/50">
                <CardContent className="py-4">
                  <div className="flex items-center gap-4">
                    <Badge
                      variant={bp.phase === "request" ? "default" : "secondary"}
                      className="text-xs"
                    >
                      {bp.phase === "request" ? "REQ" : "RES"}
                    </Badge>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant="outline" className="text-xs">
                          {bp.data.method}
                        </Badge>
                        <span className="text-sm font-mono truncate">{bp.data.url}</span>
                      </div>
                      {bp.data.status && (
                        <span className="text-xs text-muted-foreground">
                          Status: {bp.data.status}
                        </span>
                      )}
                    </div>
                    {/* TODO: modify_and_forward 액션 UI 구현 - 요청/응답 본문 편집 후 전달 기능 */}
                    <div className="flex items-center gap-1">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleForward(bp)}
                        title={t`Forward`}
                      >
                        <Play className="w-4 h-4 mr-1" />
                        <Trans>Forward</Trans>
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDrop(bp)}
                        title={t`Drop`}
                      >
                        <XCircle className="w-4 h-4 mr-1" />
                        <Trans>Drop</Trans>
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleAbort(bp)}
                        title={t`Abort`}
                        className="text-destructive hover:text-destructive"
                      >
                        <StopCircle className="w-4 h-4 mr-1" />
                        <Trans>Abort</Trans>
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}

        {/* Rules List */}
        {rules.length === 0 && !showAddForm ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <div className="text-center space-y-2">
                <h3 className="text-lg font-semibold">
                  <Trans>No breakpoint rules</Trans>
                </h3>
                <p className="text-muted-foreground">
                  <Trans>
                    Add breakpoint rules to pause HTTP requests/responses for inspection.
                  </Trans>
                </p>
                <Button className="mt-4" onClick={() => setShowAddForm(true)}>
                  <Plus className="w-4 h-4 mr-1" />
                  <Trans>Add your first rule</Trans>
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {rules.map((rule) => (
              <Card key={rule.id}>
                <CardContent className="py-4">
                  <div className="flex items-center gap-4">
                    <Switch checked={rule.enabled} onCheckedChange={() => toggleRule(rule.id)} />
                    <div
                      className={`flex-1 min-w-0 transition-opacity ${!rule.enabled ? "opacity-50" : ""}`}
                    >
                      <div className="flex items-center gap-2 mb-1">
                        <code className="text-sm font-mono bg-muted px-1.5 py-0.5 rounded truncate">
                          {rule.pattern}
                        </code>
                      </div>
                      <div className="flex items-center gap-2">
                        {rule.break_on_request && (
                          <Badge variant="default" className="text-xs">
                            <Trans>Request</Trans>
                          </Badge>
                        )}
                        {rule.break_on_response && (
                          <Badge variant="secondary" className="text-xs">
                            <Trans>Response</Trans>
                          </Badge>
                        )}
                        {!rule.break_on_request && !rule.break_on_response && (
                          <Badge variant="outline" className="text-xs">
                            <Trans>No phase selected</Trans>
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
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
