import { useCallback, useMemo } from "react";
import Editor from "@monaco-editor/react";
import { useTheme } from "next-themes";
import { Code, LayoutList } from "lucide-react";
import { useLingui } from "@lingui/react/macro";

import { cn } from "@/shared/lib";
import { Badge, TooltipProvider } from "@/shared/ui";

import { useMonacoEditor } from "../hooks";
import { FilterHelpTooltip } from "./filter-help-tooltip";
import { FilterPresetMenu } from "./filter-preset-menu";
import { Separator } from "@/shared/ui/separator";
import { serializeBuilderState } from "../lib/query-serializer";
import type { BuilderState } from "../lib/query-serializer";
import { getMonacoThemeName } from "../model/themes";

export type EditorMode = "code" | "builder";

export interface QueryFilterEditorProps {
  value: string;
  appliedValue: string;
  onChange: (value: string) => void;
  onApply: (value: string) => void;
  totalCount: number;
  filteredCount: number;
  mode: EditorMode;
  onModeChange: (mode: EditorMode) => void;
  builderState: BuilderState;
}

export const QueryFilterEditor = ({
  value,
  appliedValue,
  onChange,
  onApply,
  totalCount,
  filteredCount,
  mode,
  onModeChange,
  builderState,
}: QueryFilterEditorProps) => {
  const isDirty = value !== appliedValue;
  const { resolvedTheme } = useTheme();
  const { t } = useLingui();

  const { handleEditorDidMount } = useMonacoEditor({
    onApply,
    onChange,
    appliedValue,
  });

  const statsText = useMemo(() => {
    if (totalCount !== filteredCount) {
      return `${filteredCount} / ${totalCount}`;
    }
    return `${totalCount}`;
  }, [totalCount, filteredCount]);

  const handleSwitchToCode = useCallback(() => {
    const query = serializeBuilderState(builderState);
    onChange(query);
    onModeChange("code");
  }, [builderState, onChange, onModeChange]);

  if (mode === "builder") {
    const previewText = serializeBuilderState(builderState) || t`No conditions`;

    return (
      <div className="relative w-full h-[36px] min-w-0 flex items-center">
        <div className="absolute w-full flex-1 min-w-0 right-0">
          <div className="w-full h-[36px] flex items-center border rounded-md bg-background">
            <div className="flex items-center gap-1.5 px-2 flex-1 min-w-0">
              <LayoutList className="w-3.5 h-3.5 text-accent shrink-0" />
              <span
                className={cn(
                  "text-xs font-mono truncate",
                  serializeBuilderState(builderState)
                    ? "text-muted-foreground"
                    : "text-muted-foreground/40",
                )}
              >
                {previewText}
              </span>
            </div>

            <Separator orientation="vertical" />

            <button
              type="button"
              className="mx-1 p-1.5 hover:bg-accent/10 rounded-md transition-colors"
              onClick={handleSwitchToCode}
              title={t`Switch to Code mode`}
            >
              <Code className="w-3.5 h-3.5 text-muted-foreground" />
            </button>

            <Separator orientation="vertical" />

            <FilterPresetMenu
              currentQuery={serializeBuilderState(builderState)}
              onSelectPreset={(query) => {
                onChange(query);
                onApply(query);
              }}
            />

            <Separator orientation="vertical" />
            <Badge className="mx-2 text-[10px] font-mono shrink-0 bg-accent text-accent-foreground">
              {statsText}
            </Badge>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="relative w-full h-[36px] min-w-0 flex items-center">
      <div className="absolute w-full flex-1 min-w-0 right-0">
        <div
          className={cn(
            "w-full h-[36px] flex items-center border rounded-md transition-all bg-background",
            isDirty && "border-accent shadow-[0_0_0_1px_rgba(69,203,218,0.2)]",
          )}
        >
          <TooltipProvider>
            <FilterHelpTooltip />
          </TooltipProvider>

          <div className="flex-1 min-w-0">
            <Editor
              width="100%"
              height="24px"
              language="cheolsu-query"
              value={value}
              onChange={(value) => onChange(value || "")}
              onMount={handleEditorDidMount}
              theme={getMonacoThemeName(resolvedTheme)}
              options={{
                minimap: { enabled: false },
                lineNumbers: "off",
                glyphMargin: false,
                folding: false,
                lineDecorationsWidth: 0,
                lineNumbersMinChars: 0,
                scrollBeyondLastLine: false,
                wordWrap: "off",
                overviewRulerLanes: 0,
                hideCursorInOverviewRuler: true,
                scrollbar: {
                  vertical: "hidden",
                  horizontal: "auto",
                  horizontalScrollbarSize: 4,
                },
                automaticLayout: true,
                renderLineHighlight: "none",
                renderWhitespace: "none",
                fontFamily: "var(--font-jetbrains-mono), Consolas, Monaco, monospace",
                fontSize: 12,
                lineHeight: 24,
                suggest: {
                  showKeywords: true,
                  showSnippets: true,
                  snippetsPreventQuickSuggestions: false,
                  insertMode: "insert",
                  showInlineDetails: false,
                  preview: false,
                },
                suggestLineHeight: 22,
                suggestFontSize: 12,
                quickSuggestions: true,
                acceptSuggestionOnEnter: "on",
                suggestOnTriggerCharacters: true,
                tabCompletion: "on",
              }}
            />
          </div>

          {isDirty && (
            <div className="flex items-center gap-1 pointer-events-none mr-2">
              <span className="text-xs text-accent bg-background/95 px-2 py-0.5 rounded border border-accent/30 backdrop-blur-sm">
                <kbd className="font-mono">⌘↵</kbd>
              </span>
            </div>
          )}

          <Separator orientation="vertical" />

          <button
            type="button"
            className="mx-1 p-1.5 hover:bg-accent/10 rounded-md transition-colors"
            onClick={() => onModeChange("builder")}
            title={t`Switch to Builder mode`}
          >
            <LayoutList className="w-3.5 h-3.5 text-muted-foreground" />
          </button>

          <Separator orientation="vertical" />

          <FilterPresetMenu
            currentQuery={appliedValue}
            onSelectPreset={(query) => {
              onChange(query);
              onApply(query);
            }}
          />

          <Separator orientation="vertical" />
          <Badge className="mx-2 text-[10px] font-mono shrink-0 bg-accent text-accent-foreground">
            {statsText}
          </Badge>
        </div>
      </div>
    </div>
  );
};
