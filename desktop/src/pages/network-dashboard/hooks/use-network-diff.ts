import { useCallback, useState } from "react";

import type { HttpTransaction } from "@/entities/proxy";
import {
  diffTransactionPairs,
  type DiffTransactionPair,
  type TrafficDiff,
} from "@/shared/api/proxy";

interface UseNetworkDiffParams {
  checkedTransactionIds: Set<string>;
  checkedTransactions: HttpTransaction[];
}

export function useNetworkDiff({
  checkedTransactionIds,
  checkedTransactions,
}: UseNetworkDiffParams) {
  const [diffResult, setDiffResult] = useState<TrafficDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);

  const canCompare = checkedTransactionIds.size === 2;

  const handleCompare = useCallback(async () => {
    if (!canCompare || diffLoading) return;

    const checked = checkedTransactions;
    if (checked.length !== 2) return;

    const toPair = (tx: HttpTransaction): DiffTransactionPair => {
      const req = tx.request;
      const res = tx.response;
      return {
        request: req
          ? {
              method: req.method,
              uri: req.uri,
              headers: Object.entries(req.headers ?? {}),
              body: req.body_json != null ? JSON.stringify(req.body_json) : undefined,
              body_size: req.body_size ?? 0,
              data_type: req.data_type,
            }
          : undefined,
        response: res
          ? {
              status: res.status,
              headers: Object.entries(res.headers ?? {}),
              body: res.body_json != null ? JSON.stringify(res.body_json) : undefined,
              body_size: res.body_size ?? 0,
              data_type: res.data_type,
            }
          : undefined,
      };
    };

    try {
      setDiffLoading(true);
      const result = await diffTransactionPairs(toPair(checked[0]), toPair(checked[1]));
      setDiffResult(result);
    } catch (err) {
      console.error("Diff failed:", err);
    } finally {
      setDiffLoading(false);
    }
  }, [canCompare, diffLoading, checkedTransactions]);

  const closeDiff = useCallback(() => {
    setDiffResult(null);
  }, []);

  return {
    diffResult,
    diffLoading,
    canCompare,
    handleCompare,
    closeDiff,
  };
}
