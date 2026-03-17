import { memo, useMemo, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { Trans } from "@lingui/react/macro";
import { X, AlertTriangle } from "lucide-react";
import { Editor } from "@monaco-editor/react";
import { useTheme } from "next-themes";
import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { cn } from "@/shared/lib";
import type { GraphqlOperation } from "@/entities/graphql";

interface GraphqlOperationDetailProps {
  operation: GraphqlOperation;
  onClose: () => void;
}

type DetailTab = "query" | "variables" | "response" | "errors";

function formatTimeFull(nanos: number): string {
  const ms = nanos / 1_000_000;
  return new Date(ms).toLocaleString();
}

function formatDuration(requestNanos: number, responseNanos: number | null): string {
  if (responseNanos == null) return "-";
  const ms = (responseNanos - requestNanos) / 1_000_000;
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

const TYPE_COLORS: Record<string, string> = {
  query: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
  mutation:
    "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300",
  subscription:
    "bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300",
};

const EDITOR_OPTIONS = {
  readOnly: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  fontSize: 12,
  lineNumbers: "off" as const,
  wordWrap: "on" as const,
  automaticLayout: true,
  padding: { top: 8, bottom: 8 },
  scrollbar: {
    vertical: "auto" as const,
    horizontal: "auto" as const,
  },
  renderLineHighlight: "none" as const,
  overviewRulerLanes: 0,
  hideCursorInOverviewRuler: true,
  overviewRulerBorder: false,
  contextmenu: false,
};

export const GraphqlOperationDetail = memo(
  ({ operation, onClose }: GraphqlOperationDetailProps) => {
    const { t } = useLingui();
    const { resolvedTheme } = useTheme();

    const hasErrors =
      operation.responseErrors && operation.responseErrors.length > 0;

    const [activeTab, setActiveTab] = useState<DetailTab>("query");

    const metaItems = useMemo(() => {
      const items = [
        {
          label: t`Type`,
          value: operation.operationType.toUpperCase(),
        },
        {
          label: t`Name`,
          value: operation.operationName ?? "(anonymous)",
        },
        { label: t`Endpoint`, value: operation.endpoint },
        { label: t`Time`, value: formatTimeFull(operation.requestTime) },
        {
          label: t`Duration`,
          value: formatDuration(
            operation.requestTime,
            operation.responseTime,
          ),
        },
      ];

      if (operation.statusCode != null) {
        items.push({
          label: t`Status`,
          value: String(operation.statusCode),
        });
      }

      if (operation.batchSize != null) {
        items.push({
          label: t`Batch`,
          value: `${(operation.batchIndex ?? 0) + 1} / ${operation.batchSize}`,
        });
      }

      return items;
    }, [operation, t]);

    const formattedVariables = useMemo(() => {
      if (operation.variables == null) return null;
      try {
        return JSON.stringify(operation.variables, null, 2);
      } catch {
        return String(operation.variables);
      }
    }, [operation.variables]);

    const formattedResponseData = useMemo(() => {
      if (operation.responseData == null) return null;
      try {
        return JSON.stringify(operation.responseData, null, 2);
      } catch {
        return String(operation.responseData);
      }
    }, [operation.responseData]);

    const formattedErrors = useMemo(() => {
      if (!operation.responseErrors || operation.responseErrors.length === 0)
        return null;
      try {
        return JSON.stringify(operation.responseErrors, null, 2);
      } catch {
        return String(operation.responseErrors);
      }
    }, [operation.responseErrors]);

    const tabs: { id: DetailTab; label: string; available: boolean }[] =
      useMemo(
        () => [
          { id: "query", label: t`Query`, available: true },
          {
            id: "variables",
            label: t`Variables`,
            available: formattedVariables != null,
          },
          {
            id: "response",
            label: t`Response`,
            available: formattedResponseData != null,
          },
          {
            id: "errors",
            label: t`Errors`,
            available: hasErrors ?? false,
          },
        ],
        [t, formattedVariables, formattedResponseData, hasErrors],
      );

    const editorTheme = resolvedTheme === "dark" ? "vs-dark" : "vs";

    return (
      <div className="h-full flex flex-col bg-card select-text">
        {/* Header bar */}
        <div className="flex items-center justify-between px-3 py-1.5 border-b border-border flex-shrink-0 bg-muted/30">
          <div className="flex items-center gap-2 text-xs font-medium">
            <span
              className={cn(
                "px-1.5 py-0.5 rounded text-[10px] font-medium uppercase",
                TYPE_COLORS[operation.operationType] ?? TYPE_COLORS.query,
              )}
            >
              {operation.operationType}
            </span>
            <span className="font-mono text-foreground">
              {operation.operationName ?? "(anonymous)"}
            </span>
            {hasErrors && (
              <AlertTriangle className="w-3.5 h-3.5 text-destructive" />
            )}
          </div>
          <div className="flex items-center gap-1">
            <Tooltip>
              <TooltipTrigger render={<div />}>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0"
                  onClick={onClose}
                >
                  <X className="w-3.5 h-3.5" />
                </Button>
              </TooltipTrigger>
              <TooltipContent sideOffset={4}>
                <Trans>Close</Trans>
              </TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Meta info */}
        <div className="border-b border-border flex-shrink-0">
          <table className="w-full text-xs">
            <tbody>
              {metaItems.map((item) => (
                <tr
                  key={item.label}
                  className="border-b border-border/30 last:border-b-0"
                >
                  <td className="px-3 py-1 text-muted-foreground font-medium whitespace-nowrap w-24 align-top">
                    {item.label}
                  </td>
                  <td className="px-3 py-1 font-mono text-foreground break-all">
                    {item.value}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Tab bar */}
        <div className="flex border-b border-border flex-shrink-0 bg-muted/20">
          {tabs
            .filter((tab) => tab.available)
            .map((tab) => (
              <button
                key={tab.id}
                className={cn(
                  "px-3 py-1.5 text-xs font-medium transition-colors border-b-2",
                  activeTab === tab.id
                    ? "border-primary text-foreground"
                    : "border-transparent text-muted-foreground hover:text-foreground",
                  tab.id === "errors" && "text-destructive",
                )}
                onClick={() => setActiveTab(tab.id)}
              >
                {tab.label}
                {tab.id === "errors" && hasErrors && (
                  <span className="ml-1 text-[10px] text-destructive">
                    ({operation.responseErrors!.length})
                  </span>
                )}
              </button>
            ))}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-hidden">
          {activeTab === "query" && (
            <Editor
              height="100%"
              language="graphql"
              value={operation.query}
              theme={editorTheme}
              options={EDITOR_OPTIONS}
            />
          )}
          {activeTab === "variables" && formattedVariables && (
            <Editor
              height="100%"
              language="json"
              value={formattedVariables}
              theme={editorTheme}
              options={EDITOR_OPTIONS}
            />
          )}
          {activeTab === "response" && formattedResponseData && (
            <Editor
              height="100%"
              language="json"
              value={formattedResponseData}
              theme={editorTheme}
              options={EDITOR_OPTIONS}
            />
          )}
          {activeTab === "errors" && formattedErrors && (
            <div className="h-full flex flex-col">
              <div className="px-3 py-2 bg-destructive/10 border-b border-destructive/20 flex-shrink-0">
                <div className="flex items-center gap-2 text-xs text-destructive font-medium">
                  <AlertTriangle className="w-3.5 h-3.5" />
                  <span>
                    {operation.responseErrors!.length}{" "}
                    <Trans>error(s)</Trans>
                  </span>
                </div>
              </div>
              <div className="flex-1 overflow-hidden">
                <Editor
                  height="100%"
                  language="json"
                  value={formattedErrors}
                  theme={editorTheme}
                  options={EDITOR_OPTIONS}
                />
              </div>
            </div>
          )}
        </div>
      </div>
    );
  },
);
GraphqlOperationDetail.displayName = "GraphqlOperationDetail";
