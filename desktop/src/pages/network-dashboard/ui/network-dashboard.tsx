import { useCallback, useMemo, useState } from "react";
import { save, open, confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useLingui } from "@lingui/react/macro";

import {
  TransactionDetails,
  SequenceReplayDialog,
  ReplayDialog,
  AdvancedRepeatDialog,
} from "@/features/transaction-details";
import { buildHarLog } from "@/features/har-export";
import {
  QueryFilterEditor,
  QueryBuilder,
  type EditorMode,
  type BuilderState,
  createEmptyCondition,
  parsedQueryToBuilderState,
  serializeBuilderState,
} from "@/features/query-filter-editor";
import { parseFilterQuery } from "@/shared/lib/query-parser";
import { RuleFormDialog } from "@/features/intercept-rule-form";
import { DiffView } from "@/features/traffic-diff";

import { NetworkHeader } from "@/widgets/network-header";
import { NetworkTable } from "@/widgets/network-table";
import { HostPathTree } from "@/widgets/host-path-tree";

import { ResizableHandle, ResizablePanel, ResizablePanelGroup, Button } from "@/shared/ui";
import { useDefaultLayout } from "react-resizable-panels";
import { Play, X, GitCompareArrows } from "lucide-react";

import type { HttpTransaction, ProxyEventTuple } from "@/entities/proxy";
import {
  diffTransactionPairs,
  saveSession,
  loadSession,
  importHarFile,
  type DiffTransactionPair,
  type TrafficDiff,
} from "@/shared/api/proxy";

import { useTransactionFilters, useResizablePanelController } from "../hooks";
import { useInterceptRuleDialogStore } from "@/shared/stores";
import {
  useTransactionData,
  useTransactionActions,
  useTransactionSelection,
} from "@/shared/hooks/use-transaction-selectors";

function parseTransactionsJson(json: string): HttpTransaction[] {
  const tuples: ProxyEventTuple[] = JSON.parse(json);
  return tuples.map(([request, response]) => ({ request, response }));
}

export const NetworkDashboard = () => {
  const { transactions, selectedTransaction, pinnedTransactionIds, checkedTransactionIds, paused } =
    useTransactionData();

  const { clearTransactions, deleteTransaction, setTransactions, appendTransactions, togglePause } =
    useTransactionActions();

  const {
    toggleSelectedTransaction,
    setSelectedTransaction,
    clearSelectedTransaction,
    togglePinTransaction,
    toggleCheckTransaction,
    checkAllTransactions,
    clearCheckedTransactions,
  } = useTransactionSelection();

  const {
    filterQueryString,
    appliedQueryString,
    filteredTransactions,
    onFilterQueryChange,
    onApplyFilter,
    filteredCount,
    totalCount,
  } = useTransactionFilters({ transactions });

  const detailsPanelRef = useResizablePanelController({ isExpanded: !!selectedTransaction });

  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: "network-dashboard-layout",
    storage: localStorage,
  });

  const { t } = useLingui();

  const [editorMode, setEditorMode] = useState<EditorMode>("code");
  const [builderState, setBuilderState] = useState<BuilderState>(() => ({
    conditions: [createEmptyCondition()],
  }));

  const handleModeChange = useCallback(
    (newMode: EditorMode) => {
      if (newMode === "builder") {
        const parsed = parseFilterQuery(filterQueryString);
        const state = parsedQueryToBuilderState(parsed);
        if (state.conditions.length === 0) {
          state.conditions = [createEmptyCondition()];
        }
        setBuilderState(state);
      }
      setEditorMode(newMode);
    },
    [filterQueryString],
  );

  const handleBuilderStateChange = useCallback(
    (state: BuilderState) => {
      setBuilderState(state);
      const query = serializeBuilderState(state);
      onFilterQueryChange(query);
    },
    [onFilterQueryChange],
  );

  const handleBuilderApply = useCallback(
    (query: string) => {
      onFilterQueryChange(query);
      onApplyFilter(query);
    },
    [onFilterQueryChange, onApplyFilter],
  );

  const [sequenceReplayOpen, setSequenceReplayOpen] = useState(false);
  const [composeOpen, setComposeOpen] = useState(false);
  const [advancedRepeatTarget, setAdvancedRepeatTarget] = useState<HttpTransaction | null>(null);
  const [exporting, setExporting] = useState(false);
  const [diffResult, setDiffResult] = useState<TrafficDiff | null>(null);
  const [diffLoading, setDiffLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);

  const busy = exporting || saving || loading;

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

  const interceptRuleDialogOpen = useInterceptRuleDialogStore((s) => s.open);
  const interceptRuleInitialValues = useInterceptRuleDialogStore((s) => s.initialValues);
  const closeInterceptRuleDialog = useInterceptRuleDialogStore((s) => s.close);

  const createTransactionDeleteHandler = useCallback(
    (id: string) => () => {
      deleteTransaction(id);

      if (selectedTransaction?.request?.id === id) {
        clearSelectedTransaction();
      }
    },
    [clearSelectedTransaction, deleteTransaction, selectedTransaction],
  );

  const createTransactionPinHandler = useCallback(
    (id: string) => () => {
      togglePinTransaction(id);
    },
    [togglePinTransaction],
  );

  const createTransactionCheckHandler = useCallback(
    (id: string) => () => {
      toggleCheckTransaction(id);
    },
    [toggleCheckTransaction],
  );

  const createTransactionToggleHandler = useCallback(
    (transaction: import("@/entities/proxy").HttpTransaction) => () => {
      toggleSelectedTransaction(transaction);
    },
    [toggleSelectedTransaction],
  );

  const createTransactionSelectHandler = useCallback(
    (transaction: import("@/entities/proxy").HttpTransaction) => () => {
      setSelectedTransaction(transaction);
    },
    [setSelectedTransaction],
  );

  const handleToggleCheckAll = useCallback(() => {
    const allIds = filteredTransactions
      .map((t) => t.request?.id)
      .filter((id): id is string => !!id);
    checkAllTransactions(allIds);
  }, [filteredTransactions, checkAllTransactions]);

  const checkedTransactions = useMemo(
    () => transactions.filter((t) => t.request?.id && checkedTransactionIds.has(t.request.id)),
    [transactions, checkedTransactionIds],
  );

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

  return (
    <>
      <div className="flex-1 flex flex-col h-full overflow-x-hidden">
        <NetworkHeader
          paused={paused}
          hasTransactions={filteredTransactions.length > 0}
          exporting={busy}
          togglePause={togglePause}
          clearTransactions={clearTransactions}
          onExportHar={handleExportHar}
          onSaveSession={handleSaveSession}
          onLoadSession={handleLoadSession}
          onImportHar={handleImportHar}
          onCompose={() => setComposeOpen(true)}
          filterSlot={
            <QueryFilterEditor
              totalCount={totalCount}
              filteredCount={filteredCount}
              value={filterQueryString}
              appliedValue={appliedQueryString}
              onChange={onFilterQueryChange}
              onApply={onApplyFilter}
              mode={editorMode}
              onModeChange={handleModeChange}
              builderState={builderState}
            />
          }
        />

        {editorMode === "builder" && (
          <QueryBuilder
            builderState={builderState}
            onBuilderStateChange={handleBuilderStateChange}
            onApply={handleBuilderApply}
            onClose={() => setEditorMode("code")}
            totalCount={totalCount}
            filteredCount={filteredCount}
          />
        )}

        <div className="flex-1 flex flex-col overflow-hidden relative">
          <ResizablePanelGroup
            orientation="horizontal"
            defaultLayout={
              defaultLayout ?? {
                "host-path-tree": 25,
                "network-table": 75,
                "transaction-details": 0,
              }
            }
            onLayoutChanged={onLayoutChanged}
            className="flex-1 flex border border-b-0 shadow-[0_0_10px_0_rgba(0,0,0,0.05)] bg-background"
          >
            <ResizablePanel
              id="host-path-tree"
              className="h-full overflow-hidden"
              maxSize="40%"
              minSize="10%"
              collapsible
            >
              <HostPathTree
                transactions={filteredTransactions}
                selectedTransaction={selectedTransaction}
                createTransactionSelectHandler={createTransactionSelectHandler}
              />
            </ResizablePanel>

            <ResizableHandle withHandle />

            <ResizablePanel id="network-table" className="flex flex-1 h-full overflow-hidden">
              <NetworkTable
                transactions={filteredTransactions}
                pinnedTransactionIds={pinnedTransactionIds}
                checkedTransactionIds={checkedTransactionIds}
                selectedTransaction={selectedTransaction}
                createTransactionSelectHandler={createTransactionToggleHandler}
                createTransactionDeleteHandler={createTransactionDeleteHandler}
                createTransactionPinHandler={createTransactionPinHandler}
                createTransactionCheckHandler={createTransactionCheckHandler}
                onAdvancedRepeat={setAdvancedRepeatTarget}
                onToggleCheckAll={handleToggleCheckAll}
              />
            </ResizablePanel>

            <ResizableHandle withHandle={!!selectedTransaction} />
            <ResizablePanel
              panelRef={detailsPanelRef}
              id="transaction-details"
              maxSize="50%"
              minSize="25%"
              collapsible
              collapsedSize="0%"
              className="w-96 h-full overflow-y-auto"
            >
              {selectedTransaction && (
                <TransactionDetails
                  transaction={selectedTransaction}
                  clearSelectedTransaction={clearSelectedTransaction}
                />
              )}
            </ResizablePanel>
          </ResizablePanelGroup>

          {checkedTransactionIds.size > 0 && (
            <div className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-3 bg-primary text-primary-foreground px-4 py-2.5 rounded-lg shadow-lg z-10">
              <span className="text-sm font-medium">{checkedTransactionIds.size} selected</span>
              <Button size="sm" variant="secondary" onClick={() => setSequenceReplayOpen(true)}>
                <Play className="w-4 h-4 mr-1" />
                Replay
              </Button>
              {canCompare && (
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={handleCompare}
                  disabled={diffLoading}
                >
                  <GitCompareArrows className="w-4 h-4 mr-1" />
                  {diffLoading ? "Comparing..." : "Compare"}
                </Button>
              )}
              <Button
                size="sm"
                variant="ghost"
                className="text-primary-foreground hover:text-primary-foreground/80 hover:bg-primary/80"
                onClick={clearCheckedTransactions}
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          )}

          {diffResult && (
            <div className="absolute inset-0 z-20 bg-background border rounded-lg shadow-xl overflow-hidden">
              <DiffView diff={diffResult} onClose={() => setDiffResult(null)} />
            </div>
          )}
        </div>
      </div>

      <ReplayDialog open={composeOpen} onOpenChange={setComposeOpen} hideTrigger />

      <SequenceReplayDialog
        open={sequenceReplayOpen}
        onOpenChange={setSequenceReplayOpen}
        transactions={checkedTransactions}
        onComplete={clearCheckedTransactions}
      />

      {advancedRepeatTarget && (
        <AdvancedRepeatDialog
          open={!!advancedRepeatTarget}
          onOpenChange={(open) => {
            if (!open) setAdvancedRepeatTarget(null);
          }}
          transaction={advancedRepeatTarget}
        />
      )}

      <RuleFormDialog
        open={interceptRuleDialogOpen}
        onOpenChange={(open) => {
          if (!open) closeInterceptRuleDialog();
        }}
        editingRule={null}
        initialValues={interceptRuleInitialValues}
      />
    </>
  );
};
