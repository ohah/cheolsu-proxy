import { useState, useCallback, useEffect } from "react";
import { Loader2, Play, Repeat, CheckCircle, XCircle, Timer, Zap } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { listen } from "@tauri-apps/api/event";

import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";
import {
  advancedRepeat,
  type AdvancedRepeatParams,
  type AdvancedRepeatProgress,
  type AdvancedRepeatResult,
} from "@/shared/api/proxy";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  Button,
  Badge,
  Input,
  Separator,
} from "@/shared/ui";
import { uint8ArrayToString } from "../lib";

interface AdvancedRepeatDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  transaction: HttpTransaction;
}

function buildReplayParams(transaction: HttpTransaction) {
  const { request } = transaction;
  if (!request) return null;

  const headers = { ...request.headers };
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
  } else if (request.body_json) {
    body =
      typeof request.body_json === "string" ? request.body_json : JSON.stringify(request.body_json);
  }

  return {
    method: request.method,
    url: request.uri,
    headers,
    body,
  };
}

export function AdvancedRepeatDialog({
  open,
  onOpenChange,
  transaction,
}: AdvancedRepeatDialogProps) {
  const { t } = useLingui();
  const [iterations, setIterations] = useState(10);
  const [concurrency, setConcurrency] = useState(1);
  const [delayMs, setDelayMs] = useState(0);
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<AdvancedRepeatProgress | null>(null);
  const [result, setResult] = useState<AdvancedRepeatResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 다이얼로그 열릴 때 초기화
  useEffect(() => {
    if (open) {
      setProgress(null);
      setResult(null);
      setError(null);
    }
  }, [open]);

  const handleStart = useCallback(async () => {
    const replayParams = buildReplayParams(transaction);
    if (!replayParams) {
      setError(t`No valid request to replay`);
      return;
    }

    setLoading(true);
    setError(null);
    setResult(null);
    setProgress(null);

    const params: AdvancedRepeatParams = {
      ...replayParams,
      iterations,
      concurrency,
      delay_ms: delayMs,
    };

    // 리스너를 먼저 등록한 후 명령을 실행하여 초기 progress 이벤트 누락 방지
    let unlisten: (() => void) | null = await listen<AdvancedRepeatProgress>(
      "advanced_repeat_progress",
      (event) => {
        setProgress(event.payload);
      },
    );

    try {
      const res = await advancedRepeat(params);
      setResult(res);
    } catch (e: unknown) {
      setError(typeof e === "string" ? e : e instanceof Error ? e.message : t`Advanced repeat failed`);
    } finally {
      unlisten?.();
      unlisten = null;
      setLoading(false);
    }
  }, [transaction, iterations, concurrency, delayMs, t]);

  const handleClose = useCallback(() => {
    if (loading) return;
    onOpenChange(false);
  }, [loading, onOpenChange]);

  const progressPercent =
    progress && progress.total > 0 ? Math.round((progress.completed / progress.total) * 100) : 0;

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Repeat className="w-5 h-5" />
            <Trans>Advanced Repeat</Trans>
          </DialogTitle>
        </DialogHeader>

        {/* 요청 정보 */}
        <div className="flex items-center gap-2 p-2 rounded-md bg-muted/50 text-sm">
          <Badge variant="outline" className="font-mono text-xs">
            {transaction.request?.method}
          </Badge>
          <span className="font-mono text-xs truncate flex-1">{transaction.request?.uri}</span>
        </div>

        {!result && !loading && (
          <div className="space-y-4">
            {/* 반복 횟수 */}
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Iterations</Trans>
              </label>
              <Input
                type="number"
                min={1}
                max={10000}
                value={iterations}
                onChange={(e) => setIterations(Math.max(1, parseInt(e.target.value) || 1))}
              />
              <p className="text-xs text-muted-foreground">
                <Trans>Number of times to send the request (1-10000)</Trans>
              </p>
            </div>

            {/* 동시성 */}
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Concurrency</Trans>
              </label>
              <Input
                type="number"
                min={1}
                max={100}
                value={concurrency}
                onChange={(e) => setConcurrency(Math.max(1, parseInt(e.target.value) || 1))}
              />
              <p className="text-xs text-muted-foreground">
                <Trans>Number of simultaneous requests (1-100)</Trans>
              </p>
            </div>

            {/* 딜레이 */}
            <div className="space-y-2">
              <label className="text-sm font-medium">
                <Trans>Delay between requests (ms)</Trans>
              </label>
              <Input
                type="number"
                min={0}
                max={60000}
                value={delayMs}
                onChange={(e) => setDelayMs(Math.max(0, parseInt(e.target.value) || 0))}
              />
            </div>

            {error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {error}
              </div>
            )}
          </div>
        )}

        {/* 진행 중 */}
        {loading && (
          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">
                  <Trans>Progress</Trans>
                </span>
                <span className="font-mono">
                  {progress?.completed ?? 0} / {progress?.total ?? iterations}
                </span>
              </div>
              {/* Progress bar */}
              <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
                <div
                  className="h-full rounded-full bg-primary transition-all duration-150"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
              <div className="text-xs text-muted-foreground text-center">{progressPercent}%</div>
            </div>

            {progress && (
              <div className="flex items-center gap-4 text-sm">
                <div className="flex items-center gap-1">
                  <CheckCircle className="w-3.5 h-3.5 text-green-500" />
                  <span>{progress.success_count}</span>
                </div>
                <div className="flex items-center gap-1">
                  <XCircle className="w-3.5 h-3.5 text-destructive" />
                  <span>{progress.failure_count}</span>
                </div>
              </div>
            )}
          </div>
        )}

        {/* 결과 */}
        {result && (
          <div className="space-y-4">
            {/* 성공/실패 요약 */}
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2 text-sm">
                <CheckCircle className="w-4 h-4 text-green-500" />
                <span className="font-medium">
                  <Trans>{result.success_count} succeeded</Trans>
                </span>
              </div>
              {result.failure_count > 0 && (
                <div className="flex items-center gap-2 text-sm">
                  <XCircle className="w-4 h-4 text-destructive" />
                  <span className="font-medium">
                    <Trans>{result.failure_count} failed</Trans>
                  </span>
                </div>
              )}
            </div>

            <Separator />

            {/* 응답 시간 통계 */}
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Timer className="w-4 h-4" />
                <Trans>Response Time</Trans>
              </div>
              <div className="grid grid-cols-3 gap-3">
                <div className="p-3 rounded-md bg-muted/50 text-center">
                  <div className="text-xs text-muted-foreground">Min</div>
                  <div className="font-mono text-sm font-medium">{result.min_time_ms}ms</div>
                </div>
                <div className="p-3 rounded-md bg-muted/50 text-center">
                  <div className="text-xs text-muted-foreground">Avg</div>
                  <div className="font-mono text-sm font-medium">
                    {result.avg_time_ms.toFixed(1)}ms
                  </div>
                </div>
                <div className="p-3 rounded-md bg-muted/50 text-center">
                  <div className="text-xs text-muted-foreground">Max</div>
                  <div className="font-mono text-sm font-medium">{result.max_time_ms}ms</div>
                </div>
              </div>
            </div>

            <Separator />

            {/* 처리량 */}
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Zap className="w-4 h-4" />
                <Trans>Throughput</Trans>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div className="p-3 rounded-md bg-muted/50 text-center">
                  <div className="text-xs text-muted-foreground">
                    <Trans>Total Time</Trans>
                  </div>
                  <div className="font-mono text-sm font-medium">{result.total_time_ms}ms</div>
                </div>
                <div className="p-3 rounded-md bg-muted/50 text-center">
                  <div className="text-xs text-muted-foreground">
                    <Trans>Requests/sec</Trans>
                  </div>
                  <div className="font-mono text-sm font-medium">
                    {result.requests_per_second.toFixed(2)}
                  </div>
                </div>
              </div>
            </div>

            {/* 상태 코드 분포 */}
            {Object.keys(result.status_codes).length > 0 && (
              <>
                <Separator />
                <div className="space-y-2">
                  <div className="text-sm font-medium">
                    <Trans>Status Code Distribution</Trans>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(result.status_codes)
                      .sort(([a], [b]) => Number(a) - Number(b))
                      .map(([code, count]) => (
                        <Badge key={code} variant="outline" className="font-mono text-xs">
                          {code}: {count}
                        </Badge>
                      ))}
                  </div>
                </div>
              </>
            )}

            {error && (
              <div className="p-3 rounded-md bg-destructive/10 text-destructive text-sm">
                {error}
              </div>
            )}
          </div>
        )}

        <DialogFooter>
          {!result && !loading && (
            <Button onClick={handleStart} disabled={!transaction.request?.uri}>
              <Play className="w-4 h-4 mr-2" />
              <Trans>Start</Trans>
            </Button>
          )}
          {loading && (
            <Button disabled>
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              <Trans>Running...</Trans>
            </Button>
          )}
          {result && (
            <div className="flex gap-2">
              <Button
                variant="outline"
                onClick={() => {
                  setResult(null);
                  setProgress(null);
                  setError(null);
                }}
              >
                <Trans>Retry</Trans>
              </Button>
              <Button onClick={handleClose}>
                <Trans>Close</Trans>
              </Button>
            </div>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
