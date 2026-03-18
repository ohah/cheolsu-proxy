import { describe, test, expect, mock } from "bun:test";
import { renderHook, act } from "@testing-library/react";

import { useFilterEditor } from "./use-filter-editor";

describe("useFilterEditor", () => {
  const createParams = (overrides = {}) => ({
    filterQueryString: "",
    onFilterQueryChange: mock(() => {}),
    onApplyFilter: mock(() => {}),
    ...overrides,
  });

  test("초기 editorMode는 'code'이다", () => {
    const { result } = renderHook(() => useFilterEditor(createParams()));

    expect(result.current.editorMode).toBe("code");
  });

  test("초기 builderState에 빈 condition이 하나 있다", () => {
    const { result } = renderHook(() => useFilterEditor(createParams()));

    expect(result.current.builderState.conditions).toHaveLength(1);
  });

  test("setEditorMode로 모드를 직접 변경할 수 있다", () => {
    const { result } = renderHook(() => useFilterEditor(createParams()));

    act(() => {
      result.current.setEditorMode("builder");
    });

    expect(result.current.editorMode).toBe("builder");
  });

  test("handleModeChange로 code 모드로 전환하면 editorMode가 변경된다", () => {
    const { result } = renderHook(() => useFilterEditor(createParams()));

    act(() => {
      result.current.setEditorMode("builder");
    });

    act(() => {
      result.current.handleModeChange("code");
    });

    expect(result.current.editorMode).toBe("code");
  });

  test("handleModeChange로 builder 모드로 전환하면 builderState가 초기화된다", () => {
    const { result } = renderHook(() => useFilterEditor(createParams()));

    act(() => {
      result.current.handleModeChange("builder");
    });

    expect(result.current.editorMode).toBe("builder");
    expect(result.current.builderState.conditions.length).toBeGreaterThanOrEqual(1);
  });

  test("handleBuilderApply가 onFilterQueryChange와 onApplyFilter를 호출한다", () => {
    const onFilterQueryChange = mock(() => {});
    const onApplyFilter = mock(() => {});

    const { result } = renderHook(() =>
      useFilterEditor(
        createParams({ onFilterQueryChange, onApplyFilter }),
      ),
    );

    act(() => {
      result.current.handleBuilderApply('method="GET"');
    });

    expect(onFilterQueryChange).toHaveBeenCalledWith('method="GET"');
    expect(onApplyFilter).toHaveBeenCalledWith('method="GET"');
  });

  test("handleBuilderStateChange가 builderState를 업데이트하고 쿼리를 동기화한다", () => {
    const onFilterQueryChange = mock(() => {});

    const { result } = renderHook(() =>
      useFilterEditor(createParams({ onFilterQueryChange })),
    );

    const newState = {
      conditions: [
        {
          id: "cond_test_1",
          field: "method" as const,
          operator: "=" as const,
          values: ["GET"],
          logicalOperator: "and" as const,
        },
      ],
    };

    act(() => {
      result.current.handleBuilderStateChange(newState);
    });

    expect(result.current.builderState).toEqual(newState);
    expect(onFilterQueryChange).toHaveBeenCalled();
  });
});
