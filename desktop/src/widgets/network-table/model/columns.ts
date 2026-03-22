import { createColumnHelper, type SortingFn } from "@tanstack/react-table";
import type { TableRowData } from "./types";
import { TABLE_COLUMNS } from "./consts";

const columnHelper = createColumnHelper<TableRowData>();

const gridSizeMap = Object.fromEntries(TABLE_COLUMNS.map((c) => [c.key, c.gridSize]));

const numericTimeDiff: SortingFn<TableRowData> = (rowA, rowB) => {
  const a = typeof rowA.original.timeDiff === "number" ? rowA.original.timeDiff : 0;
  const b = typeof rowB.original.timeDiff === "number" ? rowB.original.timeDiff : 0;
  return a - b;
};

export const columns = [
  columnHelper.accessor("authority", {
    id: "path",
    header: "Path",
    sortingFn: "alphanumeric",
    meta: { gridSize: gridSizeMap.path },
  }),
  columnHelper.accessor((row) => row.transaction.request?.method ?? "", {
    id: "method",
    header: "Method",
    sortingFn: "alphanumeric",
    meta: { gridSize: gridSizeMap.method },
  }),
  columnHelper.accessor((row) => row.transaction.response?.status ?? 0, {
    id: "status",
    header: "Status",
    sortingFn: "basic",
    meta: { gridSize: gridSizeMap.status },
  }),
  columnHelper.accessor(
    (row) => (row.transaction.request?.body_size ?? 0) + (row.transaction.response?.body_size ?? 0),
    {
      id: "size",
      header: "Size",
      sortingFn: "basic",
      meta: { gridSize: gridSizeMap.size },
    },
  ),
  columnHelper.accessor("timeDiff", {
    id: "time",
    header: "Time",
    sortingFn: numericTimeDiff,
    meta: { gridSize: gridSizeMap.time },
  }),
  columnHelper.accessor((row) => row.transaction.request?.client_addr ?? "", {
    id: "client",
    header: "Client",
    sortingFn: "alphanumeric",
    meta: { gridSize: gridSizeMap.client },
  }),
  columnHelper.display({
    id: "waterfall",
    header: "Waterfall",
    enableSorting: false,
    meta: { gridSize: gridSizeMap.waterfall },
  }),
];
