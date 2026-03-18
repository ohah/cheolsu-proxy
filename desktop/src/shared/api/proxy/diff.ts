import { invoke } from "@tauri-apps/api/core";

export interface DiffTransactionData {
  method?: string;
  uri?: string;
  status?: number;
  headers: [string, string][];
  body?: string;
  body_size: number;
  data_type?: string;
}

export interface DiffTransactionPair {
  request?: DiffTransactionData;
  response?: DiffTransactionData;
}

export interface HeaderDiff {
  type: "added" | "removed" | "modified";
  key: string;
  value?: string;
  old_value?: string;
  new_value?: string;
}

export interface DiffLine {
  line_number: number;
  content: string;
}

export interface JsonDiffEntry {
  path: string;
  change_type: string;
  old_value?: string;
  new_value?: string;
}

export type BodyDiff =
  | {
      type: "text";
      additions: DiffLine[];
      deletions: DiffLine[];
      unchanged: number;
    }
  | { type: "json"; changes: JsonDiffEntry[] }
  | { type: "binary"; old_size: number; new_size: number };

export interface TransactionPartDiff {
  method_diff?: [string, string];
  url_diff?: [string, string];
  status_diff?: [number, number];
  header_diffs: HeaderDiff[];
  body_diff?: BodyDiff;
}

export interface TrafficDiff {
  request_diff?: TransactionPartDiff;
  response_diff?: TransactionPartDiff;
}

export async function diffTransactions(
  transactionA: DiffTransactionData,
  transactionB: DiffTransactionData,
): Promise<TrafficDiff> {
  return invoke("diff_transactions", { transactionA, transactionB });
}

export async function diffTransactionPairs(
  pairA: DiffTransactionPair,
  pairB: DiffTransactionPair,
): Promise<TrafficDiff> {
  return invoke("diff_transaction_pairs", { pairA, pairB });
}
