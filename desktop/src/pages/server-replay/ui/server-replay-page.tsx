import { useState, useCallback } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { Trash2, Plus, Eraser, Database } from "lucide-react";
import { toast } from "sonner";
import { useServerReplayStore, useTransactionStore } from "@/shared/stores";
import { Card, CardContent, Badge, Button, Switch } from "@/shared/ui";
import { getStatusColor } from "@/entities/transaction";
import type { HttpTransaction } from "@/entities/proxy";
import { transactionToReplayEntry } from "@/shared/lib";

function truncateUrl(url: string, max = 80): string {
  return url.length > max ? `${url.slice(0, max)}...` : url;
}

export function ServerReplayPage() {
  const { t } = useLingui();
  const { entries, enabled, addEntry, removeEntry, clearEntries, setEnabled } =
    useServerReplayStore();
  const { transactions } = useTransactionStore();
  const [showPicker, setShowPicker] = useState(false);

  const handleAddFromTransaction = useCallback(
    (tx: HttpTransaction) => {
      const entry = transactionToReplayEntry(tx);
      if (!entry) {
        toast.error(t`Cannot add: no response data`);
        return;
      }
      addEntry(entry);
      toast.success(t`Added to server replay`);
    },
    [addEntry, t],
  );

  const handleClearAll = () => {
    clearEntries();
    toast.success(t`All entries cleared`);
  };

  // 응답이 있는 트랜잭션만 필터
  const eligibleTransactions = transactions.filter((tx) => tx.request && tx.response);

  return (
    <div className="flex-1 flex flex-col h-full overflow-auto">
      <div className="p-6 space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Server Replay</Trans>
            </h1>
            <p className="text-muted-foreground">
              <Trans>
                Return cached responses for matching requests instead of forwarding to the server
              </Trans>
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <Switch checked={enabled} onCheckedChange={setEnabled} />
              <span className="text-sm font-medium">
                {enabled ? <Trans>Enabled</Trans> : <Trans>Disabled</Trans>}
              </span>
            </div>
            <Badge variant="outline" className="text-sm">
              {entries.length} <Trans>entries</Trans>
            </Badge>
            {entries.length > 0 && (
              <Button variant="outline" size="sm" onClick={handleClearAll}>
                <Eraser className="w-4 h-4 mr-1" />
                <Trans>Clear All</Trans>
              </Button>
            )}
            <Button size="sm" onClick={() => setShowPicker(!showPicker)}>
              <Plus className="w-4 h-4 mr-1" />
              <Trans>Add from Traffic</Trans>
            </Button>
          </div>
        </div>

        {/* 트래픽에서 선택 패널 */}
        {showPicker && (
          <Card>
            <CardContent className="py-4">
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-semibold">
                  <Trans>Select from captured traffic</Trans>
                </h3>
                <Badge variant="outline" className="text-xs">
                  {eligibleTransactions.length} <Trans>available</Trans>
                </Badge>
              </div>
              {eligibleTransactions.length === 0 ? (
                <p className="text-sm text-muted-foreground py-4 text-center">
                  <Trans>
                    No captured traffic with responses available. Capture some traffic first.
                  </Trans>
                </p>
              ) : (
                <div className="max-h-64 overflow-y-auto space-y-1">
                  {eligibleTransactions.map((tx) => {
                    const req = tx.request!;
                    const res = tx.response!;
                    const alreadyAdded = entries.some(
                      (e) => e.method === req.method && e.url === req.uri,
                    );
                    return (
                      <div
                        key={req.id}
                        className="flex items-center gap-3 px-3 py-2 rounded-md hover:bg-muted/50 group"
                      >
                        <Badge variant="outline" className="text-xs w-16 justify-center">
                          {req.method}
                        </Badge>
                        <Badge
                          variant="outline"
                          className={`text-xs ${getStatusColor(res.status)}`}
                        >
                          {res.status}
                        </Badge>
                        <span className="text-xs font-mono text-muted-foreground flex-1 truncate">
                          {truncateUrl(req.uri)}
                        </span>
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={alreadyAdded}
                          onClick={() => handleAddFromTransaction(tx)}
                          className="opacity-0 group-hover:opacity-100"
                        >
                          {alreadyAdded ? (
                            <Trans>Added</Trans>
                          ) : (
                            <>
                              <Plus className="w-3.5 h-3.5 mr-1" />
                              <Trans>Add</Trans>
                            </>
                          )}
                        </Button>
                      </div>
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        )}

        {/* 등록된 엔트리 목록 */}
        {entries.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <Database className="w-12 h-12 text-muted-foreground/50 mb-4" />
              <div className="text-center space-y-2">
                <h3 className="text-lg font-semibold">
                  <Trans>No replay entries</Trans>
                </h3>
                <p className="text-muted-foreground">
                  <Trans>
                    Add captured traffic to replay stored responses for matching requests.
                  </Trans>
                </p>
                <Button className="mt-4" onClick={() => setShowPicker(true)}>
                  <Plus className="w-4 h-4 mr-1" />
                  <Trans>Add from Traffic</Trans>
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-3">
            {entries.map((entry) => (
              <Card key={entry.id} className={!enabled ? "opacity-50" : ""}>
                <CardContent className="py-4">
                  <div className="flex items-center gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant="outline" className="text-xs w-16 justify-center">
                          {entry.method}
                        </Badge>
                        <Badge
                          variant="outline"
                          className={`text-xs ${getStatusColor(entry.status)}`}
                        >
                          {entry.status}
                        </Badge>
                        <code className="text-xs text-muted-foreground truncate">{entry.url}</code>
                      </div>
                      <div className="flex items-center gap-2 text-xs text-muted-foreground">
                        <span>
                          {Object.keys(entry.headers).length} <Trans>headers</Trans>
                        </span>
                        {entry.body && (
                          <span>
                            {entry.body.length} <Trans>bytes body</Trans>
                          </span>
                        )}
                      </div>
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => removeEntry(entry.id)}
                      title={t`Remove entry`}
                      className="text-destructive hover:text-destructive"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
