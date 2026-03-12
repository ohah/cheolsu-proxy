import { useState, useCallback } from "react";
import { Loader2, Play, CheckCircle, XCircle } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";
import {
  replaySequence,
  type ReplayRequestParams,
  type SequenceReplayResult,
} from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  Button,
  Badge,
  ScrollArea,
  Separator,
} from "@/shared/ui";
import { sanitizeHopByHopHeaders } from "@/shared/lib/http-headers";
import { getStatusColor } from "@/entities/transaction";
import { uint8ArrayToString } from "../lib";

interface SequenceReplayDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  transactions: HttpTransaction[];
  onComplete: () => void;
}

function transactionToReplayParams(transaction: HttpTransaction): ReplayRequestParams | null {
  const { request } = transaction;
  if (!request) return null;

  const headers = sanitizeHopByHopHeaders(request.headers);

  let body: string | undefined;
  if (request.body && request.data_type && isTextBasedDataType(request.data_type)) {
    body = uint8ArrayToString(request.body, request.data_type);
  }

  return {
    method: request.method,
    url: request.uri,
    headers,
    body,
  };
}

export function SequenceReplayDialog({
  open,
  onOpenChange,
  transactions,
  onComplete,
}: SequenceReplayDialogProps) {
  const { t } = useLingui();
  const [loading, setLoading] = useState(false);
  const [results, setResults] = useState<SequenceReplayResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleReplay = useCallback(async () => {
    setLoading(true);
    setError(null);
    setResults(null);

    const requests: ReplayRequestParams[] = [];
    for (const t of transactions) {
      const params = transactionToReplayParams(t);
      if (params) requests.push(params);
    }

    if (requests.length === 0) {
      setError(t`No valid requests to replay`);
      setLoading(false);
      return;
    }

    try {
      const res = await replaySequence(requests);
      setResults(res);
    } catch (e: unknown) {
      setError(
        typeof e === "string" ? e : e instanceof Error ? e.message : t`Sequence replay failed`,
      );
    } finally {
      setLoading(false);
    }
  }, [transactions]);

  const handleClose = useCallback(() => {
    onOpenChange(false);
    setResults(null);
    setError(null);
    onComplete();
  }, [onOpenChange, onComplete]);

  const successCount = results?.filter((r) => r.response).length ?? 0;
  const failCount = results?.filter((r) => r.error).length ?? 0;

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-4xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            <Trans>Sequence Replay ({transactions.length} requests)</Trans>
          </DialogTitle>
        </DialogHeader>

        {!results && !loading && (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              <Trans>{transactions.length} requests will be sent sequentially.</Trans>
            </p>
            <ScrollArea className="max-h-[300px]">
              <div className="space-y-2">
                {transactions.map((t, i) => (
                  <div
                    key={t.request?.id ?? i}
                    className="flex items-center gap-3 p-2 rounded-md bg-muted/50 text-sm"
                  >
                    <span className="text-muted-foreground w-6 text-right">{i + 1}.</span>
                    <Badge variant="outline" className="font-mono text-xs">
                      {t.request?.method}
                    </Badge>
                    <span className="font-mono text-xs truncate flex-1">{t.request?.uri}</span>
                  </div>
                ))}
              </div>
            </ScrollArea>

            {error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {error}
              </div>
            )}
          </div>
        )}

        {loading && (
          <div className="flex flex-col items-center justify-center py-12 gap-4">
            <Loader2 className="w-8 h-8 animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">
              <Trans>Sending requests sequentially...</Trans>
            </p>
          </div>
        )}

        {results && (
          <div className="space-y-4">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-1 text-sm">
                <CheckCircle className="w-4 h-4 text-green-500" />
                <span>
                  <Trans>{successCount} succeeded</Trans>
                </span>
              </div>
              {failCount > 0 && (
                <div className="flex items-center gap-1 text-sm">
                  <XCircle className="w-4 h-4 text-destructive" />
                  <span>
                    <Trans>{failCount} failed</Trans>
                  </span>
                </div>
              )}
              {results.length > 0 && results[0].response && (
                <span className="text-xs text-muted-foreground">
                  <Trans>
                    Total {results.reduce((sum, r) => sum + (r.response?.elapsed_ms ?? 0), 0)}ms
                  </Trans>
                </span>
              )}
            </div>

            <Separator />

            <ScrollArea className="max-h-[400px]">
              <div className="space-y-2">
                {results.map((r) => (
                  <div
                    key={r.index}
                    className="flex items-center gap-3 p-2 rounded-md bg-muted/50 text-sm"
                  >
                    <span className="text-muted-foreground w-6 text-right">{r.index + 1}.</span>
                    {r.response ? (
                      <>
                        <Badge
                          variant="outline"
                          className={`text-xs ${getStatusColor(r.response.status)}`}
                        >
                          {r.response.status}
                        </Badge>
                        <Badge variant="outline" className="font-mono text-xs">
                          {r.method}
                        </Badge>
                        <span className="font-mono text-xs truncate flex-1">{r.url}</span>
                        <span className="text-xs text-muted-foreground">
                          {r.response.elapsed_ms}ms
                        </span>
                      </>
                    ) : (
                      <>
                        <XCircle className="w-4 h-4 text-destructive" />
                        <Badge variant="outline" className="font-mono text-xs">
                          {r.method}
                        </Badge>
                        <span className="font-mono text-xs truncate flex-1">{r.url}</span>
                        <span className="text-xs text-destructive truncate max-w-[200px]">
                          {r.error}
                        </span>
                      </>
                    )}
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>
        )}

        <DialogFooter>
          {!results ? (
            <Button onClick={handleReplay} disabled={loading}>
              {loading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  <Trans>Sending...</Trans>
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 mr-2" />
                  <Trans>Start Replay</Trans>
                </>
              )}
            </Button>
          ) : (
            <Button onClick={handleClose}>
              <Trans>Close</Trans>
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
