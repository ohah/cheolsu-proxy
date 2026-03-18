import { describe, test, expect } from "bun:test";
import { renderHook } from "@testing-library/react";

import { useNetworkDiff } from "./use-network-diff";

describe("useNetworkDiff", () => {
  test("초기 상태가 올바르다", () => {
    const { result } = renderHook(() =>
      useNetworkDiff({
        checkedTransactionIds: new Set<string>(),
        checkedTransactions: [],
      }),
    );

    expect(result.current.diffResult).toBeNull();
    expect(result.current.diffLoading).toBe(false);
    expect(result.current.canCompare).toBe(false);
  });

  test("체크된 트랜잭션이 2개일 때 canCompare가 true이다", () => {
    const { result } = renderHook(() =>
      useNetworkDiff({
        checkedTransactionIds: new Set(["tx-1", "tx-2"]),
        checkedTransactions: [],
      }),
    );

    expect(result.current.canCompare).toBe(true);
  });

  test("체크된 트랜잭션이 1개일 때 canCompare가 false이다", () => {
    const { result } = renderHook(() =>
      useNetworkDiff({
        checkedTransactionIds: new Set(["tx-1"]),
        checkedTransactions: [],
      }),
    );

    expect(result.current.canCompare).toBe(false);
  });

  test("체크된 트랜잭션이 3개일 때 canCompare가 false이다", () => {
    const { result } = renderHook(() =>
      useNetworkDiff({
        checkedTransactionIds: new Set(["tx-1", "tx-2", "tx-3"]),
        checkedTransactions: [],
      }),
    );

    expect(result.current.canCompare).toBe(false);
  });

  test("체크된 트랜잭션이 0개일 때 canCompare가 false이다", () => {
    const { result } = renderHook(() =>
      useNetworkDiff({
        checkedTransactionIds: new Set(),
        checkedTransactions: [],
      }),
    );

    expect(result.current.canCompare).toBe(false);
  });

  test("params 변경 시 canCompare가 올바르게 업데이트된다", () => {
    const { result, rerender } = renderHook(
      (props: { ids: Set<string> }) =>
        useNetworkDiff({
          checkedTransactionIds: props.ids,
          checkedTransactions: [],
        }),
      {
        initialProps: { ids: new Set<string>() },
      },
    );

    expect(result.current.canCompare).toBe(false);

    rerender({ ids: new Set(["tx-1", "tx-2"]) });
    expect(result.current.canCompare).toBe(true);

    rerender({ ids: new Set(["tx-1"]) });
    expect(result.current.canCompare).toBe(false);
  });
});
