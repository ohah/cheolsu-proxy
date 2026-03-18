import { useCallback, useState } from "react";

import {
  type EditorMode,
  type BuilderState,
  createEmptyCondition,
  parsedQueryToBuilderState,
  serializeBuilderState,
} from "@/features/query-filter-editor";
import { parseFilterQuery } from "@/shared/lib/query-parser";

interface UseFilterEditorParams {
  filterQueryString: string;
  onFilterQueryChange: (query: string) => void;
  onApplyFilter: (query: string) => void;
}

export function useFilterEditor({
  filterQueryString,
  onFilterQueryChange,
  onApplyFilter,
}: UseFilterEditorParams) {
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

  return {
    editorMode,
    setEditorMode,
    builderState,
    handleModeChange,
    handleBuilderStateChange,
    handleBuilderApply,
  };
}
