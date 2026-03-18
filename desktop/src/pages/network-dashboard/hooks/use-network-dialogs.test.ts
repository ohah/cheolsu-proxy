import { describe, test, expect } from "bun:test";
import { renderHook, act } from "@testing-library/react";

import { useNetworkDialogs } from "./use-network-dialogs";

describe("useNetworkDialogs", () => {
  test("초기 상태에서 모든 다이얼로그가 닫혀 있다", () => {
    const { result } = renderHook(() => useNetworkDialogs());

    expect(result.current.sequenceReplayOpen).toBe(false);
    expect(result.current.composeOpen).toBe(false);
    expect(result.current.advancedRepeatTarget).toBeNull();
  });

  test("sequenceReplayOpen을 열고 닫을 수 있다", () => {
    const { result } = renderHook(() => useNetworkDialogs());

    act(() => {
      result.current.setSequenceReplayOpen(true);
    });
    expect(result.current.sequenceReplayOpen).toBe(true);

    act(() => {
      result.current.setSequenceReplayOpen(false);
    });
    expect(result.current.sequenceReplayOpen).toBe(false);
  });

  test("composeOpen을 열고 닫을 수 있다", () => {
    const { result } = renderHook(() => useNetworkDialogs());

    act(() => {
      result.current.setComposeOpen(true);
    });
    expect(result.current.composeOpen).toBe(true);

    act(() => {
      result.current.setComposeOpen(false);
    });
    expect(result.current.composeOpen).toBe(false);
  });

  test("advancedRepeatTarget을 설정하고 해제할 수 있다", () => {
    const { result } = renderHook(() => useNetworkDialogs());

    const mockTransaction = { id: "tx-1" } as Parameters<
      typeof result.current.setAdvancedRepeatTarget
    >[0];

    act(() => {
      result.current.setAdvancedRepeatTarget(mockTransaction);
    });
    expect(result.current.advancedRepeatTarget).toBe(mockTransaction);

    act(() => {
      result.current.setAdvancedRepeatTarget(null);
    });
    expect(result.current.advancedRepeatTarget).toBeNull();
  });

  test("각 다이얼로그 상태는 독립적으로 동작한다", () => {
    const { result } = renderHook(() => useNetworkDialogs());

    act(() => {
      result.current.setSequenceReplayOpen(true);
    });

    expect(result.current.sequenceReplayOpen).toBe(true);
    expect(result.current.composeOpen).toBe(false);
    expect(result.current.advancedRepeatTarget).toBeNull();

    act(() => {
      result.current.setComposeOpen(true);
    });

    expect(result.current.sequenceReplayOpen).toBe(true);
    expect(result.current.composeOpen).toBe(true);
    expect(result.current.advancedRepeatTarget).toBeNull();
  });
});
