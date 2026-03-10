import { useMemo } from "react";

import { parseUrlencoded, type UrlencodedField } from "../lib/form-data-parser";

interface UrlencodedPreviewProps {
  data: Uint8Array | null;
}

const FieldRow = ({ field, index }: { field: UrlencodedField; index: number }) => (
  <tr className="border-b last:border-b-0 hover:bg-muted/50">
    <td className="px-3 py-2 text-xs text-muted-foreground text-right align-top w-8">
      {index + 1}
    </td>
    <td className="px-3 py-2 font-mono text-sm align-top text-blue-600 dark:text-blue-400 break-all">
      {field.key || "(empty)"}
    </td>
    <td className="px-3 py-2 font-mono text-sm align-top text-foreground break-all">
      {field.value.length > 500 ? field.value.slice(0, 500) + "..." : field.value}
    </td>
  </tr>
);

export const UrlencodedPreview = ({ data }: UrlencodedPreviewProps) => {
  const fields = useMemo<UrlencodedField[]>(() => {
    if (!data || data.length === 0) return [];
    return parseUrlencoded(data);
  }, [data]);

  if (fields.length === 0) {
    return (
      <div className="h-[calc(100vh-300px)] border rounded-md flex items-center justify-center text-muted-foreground text-sm">
        No form fields found
      </div>
    );
  }

  return (
    <div className="h-[calc(100vh-300px)] border rounded-md overflow-auto">
      {/* 헤더 */}
      <div className="flex items-center gap-3 p-3 border-b bg-muted/30 text-xs text-muted-foreground">
        <span className="font-medium text-foreground">application/x-www-form-urlencoded</span>
        <span>{fields.length} parameters</span>
      </div>

      {/* 테이블 */}
      <table className="w-full">
        <thead>
          <tr className="border-b bg-muted/20">
            <th className="px-3 py-1.5 text-right text-xs font-medium text-muted-foreground w-8">
              #
            </th>
            <th className="px-3 py-1.5 text-left text-xs font-medium text-muted-foreground w-1/3">
              Key
            </th>
            <th className="px-3 py-1.5 text-left text-xs font-medium text-muted-foreground">
              Value
            </th>
          </tr>
        </thead>
        <tbody>
          {fields.map((field, i) => (
            <FieldRow key={`${field.key}-${i}`} field={field} index={i} />
          ))}
        </tbody>
      </table>
    </div>
  );
};
