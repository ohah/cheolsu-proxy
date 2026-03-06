import { useState, useCallback } from "react";
import { Loader2, Play, CheckCircle, XCircle } from "lucide-react";

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

  const headers = { ...request.headers };
  // hop-by-hop 헤더 제거
  for (const key of [
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
  ]) {
    delete headers[key];
    delete headers[key.charAt(0).toUpperCase() + key.slice(1)];
  }

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
      setError("리플레이할 유효한 요청이 없습니다");
      setLoading(false);
      return;
    }

    try {
      const res = await replaySequence(requests);
      setResults(res);
    } catch (e: any) {
      setError(typeof e === "string" ? e : e.message || "시퀀스 리플레이 실패");
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
      <DialogContent className="sm:max-w-2xl max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Sequence Replay ({transactions.length} requests)</DialogTitle>
        </DialogHeader>

        {!results && !loading && (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              {transactions.length}개의 요청을 순서대로 재전송합니다.
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
            <p className="text-sm text-muted-foreground">요청을 순서대로 전송 중...</p>
          </div>
        )}

        {results && (
          <div className="space-y-4">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-1 text-sm">
                <CheckCircle className="w-4 h-4 text-green-500" />
                <span>{successCount} 성공</span>
              </div>
              {failCount > 0 && (
                <div className="flex items-center gap-1 text-sm">
                  <XCircle className="w-4 h-4 text-destructive" />
                  <span>{failCount} 실패</span>
                </div>
              )}
              {results.length > 0 && results[0].response && (
                <span className="text-xs text-muted-foreground">
                  총 {results.reduce((sum, r) => sum + (r.response?.elapsed_ms ?? 0), 0)}
                  ms
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
                  Sending...
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 mr-2" />
                  Start Replay
                </>
              )}
            </Button>
          ) : (
            <Button onClick={handleClose}>Close</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
