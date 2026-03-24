import { useCallback, useState } from "react";
import { save, open, confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useLingui } from "@lingui/react/macro";

import { buildHarLog } from "@/features/har-export";
import type { HttpTransaction, ProxyEventPayload } from "@/entities/proxy";
import { saveSession, loadSession, importHarFile } from "@/shared/api/proxy";

function parseTransactionsJson(json: string): HttpTransaction[] {
  const items: ProxyEventPayload[] = JSON.parse(json);
  return items;
}

interface UseNetworkExportParams {
  transactions: HttpTransaction[];
  filteredTransactions: HttpTransaction[];
  checkedTransactions: HttpTransaction[];
  appliedQueryString: string;
  setTransactions: (txs: HttpTransaction[]) => void;
  appendTransactions: (txs: HttpTransaction[]) => void;
}

export function useNetworkExport({
  transactions,
  filteredTransactions,
  checkedTransactions,
  appliedQueryString,
  setTransactions,
  appendTransactions,
}: UseNetworkExportParams) {
  const { t } = useLingui();

  const [exporting, setExporting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);

  const busy = exporting || saving || loading;

  const mergeOrReplaceTransactions = useCallback(
    async (loaded: HttpTransaction[]) => {
      if (transactions.length > 0) {
        const shouldReplace = await confirm(
          t`Replace ${transactions.length} existing transactions with ${loaded.length} new ones? Cancel to append instead.`,
          { title: t`Load transactions`, kind: "warning" },
        );
        if (shouldReplace) {
          setTransactions(loaded);
        } else {
          appendTransactions(loaded);
        }
      } else {
        setTransactions(loaded);
      }
    },
    [transactions.length, setTransactions, appendTransactions, t],
  );

  const handleExportHar = useCallback(async () => {
    if (filteredTransactions.length === 0 || busy) return;

    try {
      setExporting(true);

      const filePath = await save({
        defaultPath: "traffic.har",
        filters: [{ name: "HAR", extensions: ["har"] }],
      });
      if (!filePath) return;

      const har = await buildHarLog(filteredTransactions);
      const content = JSON.stringify(har, null, 2);
      await invoke("export_har_file", { path: filePath, content });
      toast.success(t`HAR exported successfully`);
    } catch (err) {
      console.error("HAR export failed:", err);
      toast.error(t`HAR export failed`);
    } finally {
      setExporting(false);
    }
  }, [filteredTransactions, busy, t]);

  const handleSaveSession = useCallback(async () => {
    if (filteredTransactions.length === 0 || busy) return;

    try {
      setSaving(true);

      const filePath = await save({
        defaultPath: "session.cheolsu",
        filters: [{ name: "Cheolsu Session", extensions: ["cheolsu", "cheolsu.gz"] }],
      });
      if (!filePath) return;

      const tuples = filteredTransactions.map((tx) => [tx.request, tx.response]);
      const transactionsJson = JSON.stringify(tuples);
      await saveSession(filePath, transactionsJson);
      toast.success(t`Session saved successfully`);
    } catch (err) {
      console.error("Session save failed:", err);
      toast.error(t`Session save failed`);
    } finally {
      setSaving(false);
    }
  }, [filteredTransactions, busy, t]);

  const handleLoadSession = useCallback(async () => {
    if (loading) return;

    try {
      const filePath = await open({
        filters: [{ name: "Cheolsu Session", extensions: ["cheolsu", "cheolsu.gz"] }],
        multiple: false,
      });
      if (!filePath) return;

      setLoading(true);
      const result = await loadSession(filePath);
      const loaded = parseTransactionsJson(result.transactions_json);
      await mergeOrReplaceTransactions(loaded);
      toast.success(t`Session loaded successfully`);
    } catch (err) {
      console.error("Session load failed:", err);
      toast.error(t`Session load failed`);
    } finally {
      setLoading(false);
    }
  }, [loading, mergeOrReplaceTransactions, t]);

  const handleImportHar = useCallback(async () => {
    if (loading) return;

    try {
      const filePath = await open({
        filters: [{ name: "HAR", extensions: ["har"] }],
        multiple: false,
      });
      if (!filePath) return;

      setLoading(true);
      const json = await importHarFile(filePath);
      const loaded = parseTransactionsJson(json);
      await mergeOrReplaceTransactions(loaded);
      toast.success(t`HAR imported successfully`);
    } catch (err) {
      console.error("HAR import failed:", err);
      toast.error(t`HAR import failed`);
    } finally {
      setLoading(false);
    }
  }, [loading, mergeOrReplaceTransactions, t]);

  const handleExportOpenApi = useCallback(async () => {
    if (transactions.length === 0 || busy) return;

    // 우선순위: 체크된 것 > 필터된 것 > 전체
    const targetTransactions =
      checkedTransactions.length > 0
        ? checkedTransactions
        : appliedQueryString
          ? filteredTransactions
          : transactions;

    if (targetTransactions.length === 0) return;

    try {
      setExporting(true);

      const filePath = await save({
        defaultPath: "openapi.json",
        filters: [{ name: "OpenAPI Spec", extensions: ["json", "yaml"] }],
      });
      if (!filePath) return;

      const content = await invoke<string>("generate_openapi_from_transactions", {
        transactionsJson: JSON.stringify(targetTransactions.map((tx) => [tx.request, tx.response])),
      });
      await invoke("export_har_file", { path: filePath, content });
      toast.success(
        t`OpenAPI spec exported successfully (${targetTransactions.length} transactions)`,
      );
    } catch (_err) {
      toast.error(t`OpenAPI export failed`);
    } finally {
      setExporting(false);
    }
  }, [transactions, filteredTransactions, checkedTransactions, appliedQueryString, busy, t]);

  return {
    busy,
    handleExportHar,
    handleSaveSession,
    handleLoadSession,
    handleImportHar,
    handleExportOpenApi,
  };
}
