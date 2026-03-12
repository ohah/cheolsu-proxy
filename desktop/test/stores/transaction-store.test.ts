import { describe, test, expect, beforeEach } from "bun:test";
import "../../test/mocks/tauri";

import type { HttpTransaction } from "../../src/entities/proxy";

async function getStore() {
  const { useTransactionStore } = await import("../../src/shared/stores/transaction-store");
  return useTransactionStore;
}

function createTransaction(id: string): HttpTransaction {
  return {
    request: {
      id,
      method: "GET",
      uri: `http://example.com/${id}`,
      version: "HTTP/1.1",
      headers: {},
      body: null,
      time: Date.now(),
      data_type: "text",
      body_json: null,
      mime_type: "text/plain",
      monaco_language: "plaintext",
      file_path: null,
      body_size: 0,
    },
    response: null,
  } as HttpTransaction;
}

describe("transaction-store", () => {
  let store: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    store = await getStore();
    store.setState({
      transactions: [],
      transactionIds: new Set(),
      selectedTransaction: null,
      pinnedTransactionIds: new Set(),
      checkedTransactionIds: new Set(),
      paused: false,
    });
  });

  test("초기 상태", () => {
    const state = store.getState();
    expect(state.transactions).toEqual([]);
    expect(state.selectedTransaction).toBeNull();
    expect(state.pinnedTransactionIds.size).toBe(0);
    expect(state.checkedTransactionIds.size).toBe(0);
    expect(state.paused).toBe(false);
  });

  test("addTransaction: 트랜잭션 추가", () => {
    const tx = createTransaction("tx-1");
    store.getState().addTransaction(tx);
    expect(store.getState().transactions).toHaveLength(1);
    expect(store.getState().transactions[0].request?.id).toBe("tx-1");
  });

  test("addTransaction: 중복 ID 무시", () => {
    const tx = createTransaction("tx-1");
    store.getState().addTransaction(tx);
    store.getState().addTransaction(tx);
    expect(store.getState().transactions).toHaveLength(1);
  });

  test("clearTransactions: 모든 상태 초기화", () => {
    const tx = createTransaction("tx-1");
    store.getState().addTransaction(tx);
    store.getState().setSelectedTransaction(tx);
    store.getState().togglePinTransaction("tx-1");
    store.getState().toggleCheckTransaction("tx-1");

    store.getState().clearTransactions();
    const state = store.getState();
    expect(state.transactions).toEqual([]);
    expect(state.selectedTransaction).toBeNull();
    expect(state.pinnedTransactionIds.size).toBe(0);
    expect(state.checkedTransactionIds.size).toBe(0);
  });

  test("deleteTransaction: 특정 ID 삭제", () => {
    store.getState().addTransaction(createTransaction("tx-1"));
    store.getState().addTransaction(createTransaction("tx-2"));
    store.getState().togglePinTransaction("tx-1");
    store.getState().toggleCheckTransaction("tx-1");

    store.getState().deleteTransaction("tx-1");
    const state = store.getState();
    expect(state.transactions).toHaveLength(1);
    expect(state.transactions[0].request?.id).toBe("tx-2");
    expect(state.pinnedTransactionIds.has("tx-1")).toBe(false);
    expect(state.checkedTransactionIds.has("tx-1")).toBe(false);
  });

  test("toggleSelectedTransaction: 토글 동작", () => {
    const tx = createTransaction("tx-1");
    store.getState().addTransaction(tx);

    store.getState().toggleSelectedTransaction(tx);
    expect(store.getState().selectedTransaction?.request?.id).toBe("tx-1");

    store.getState().toggleSelectedTransaction(tx);
    expect(store.getState().selectedTransaction).toBeNull();
  });

  test("togglePinTransaction: 핀 토글", () => {
    store.getState().togglePinTransaction("tx-1");
    expect(store.getState().pinnedTransactionIds.has("tx-1")).toBe(true);

    store.getState().togglePinTransaction("tx-1");
    expect(store.getState().pinnedTransactionIds.has("tx-1")).toBe(false);
  });

  test("toggleCheckTransaction: 체크 토글", () => {
    store.getState().toggleCheckTransaction("tx-1");
    expect(store.getState().checkedTransactionIds.has("tx-1")).toBe(true);

    store.getState().toggleCheckTransaction("tx-1");
    expect(store.getState().checkedTransactionIds.has("tx-1")).toBe(false);
  });

  test("checkAllTransactions: 전체 선택/해제", () => {
    const ids = ["tx-1", "tx-2", "tx-3"];

    // 전체 선택
    store.getState().checkAllTransactions(ids);
    expect(store.getState().checkedTransactionIds.size).toBe(3);

    // 이미 전체 선택됨 → 전체 해제
    store.getState().checkAllTransactions(ids);
    expect(store.getState().checkedTransactionIds.size).toBe(0);
  });

  test("checkAllTransactions: 부분 선택 → 전체 선택", () => {
    store.getState().toggleCheckTransaction("tx-1");
    store.getState().checkAllTransactions(["tx-1", "tx-2", "tx-3"]);
    expect(store.getState().checkedTransactionIds.size).toBe(3);
  });

  test("togglePause: 일시정지 토글", () => {
    expect(store.getState().paused).toBe(false);
    store.getState().togglePause();
    expect(store.getState().paused).toBe(true);
    store.getState().togglePause();
    expect(store.getState().paused).toBe(false);
  });

  test("clearSelectedTransaction: 선택 해제", () => {
    store.getState().setSelectedTransaction(createTransaction("tx-1"));
    store.getState().clearSelectedTransaction();
    expect(store.getState().selectedTransaction).toBeNull();
  });

  test("clearCheckedTransactions: 체크 해제", () => {
    store.getState().toggleCheckTransaction("tx-1");
    store.getState().toggleCheckTransaction("tx-2");
    store.getState().clearCheckedTransactions();
    expect(store.getState().checkedTransactionIds.size).toBe(0);
  });
});
