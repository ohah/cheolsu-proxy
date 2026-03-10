import { useMemo } from "react";
import { FileIcon, TextIcon } from "lucide-react";

import { formatBytes } from "@/shared/lib";
import {
  parseMultipartFormData,
  extractBoundary,
  type MultipartField,
} from "../lib/form-data-parser";

interface MultipartPreviewProps {
  data: Uint8Array | null;
  contentType: string;
}

const FieldRow = ({ field }: { field: MultipartField }) => (
  <tr className="border-b last:border-b-0 hover:bg-muted/50">
    <td className="px-3 py-2 font-mono text-sm align-top">
      <div className="flex items-center gap-1.5">
        {field.isFile ? (
          <FileIcon className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
        ) : (
          <TextIcon className="w-3.5 h-3.5 text-muted-foreground flex-shrink-0" />
        )}
        <span className="text-blue-600 dark:text-blue-400">{field.name || "(empty)"}</span>
      </div>
    </td>
    <td className="px-3 py-2 font-mono text-sm align-top break-all">
      {field.isFile ? (
        <div className="flex flex-col gap-0.5">
          <span className="text-amber-600 dark:text-amber-400">{field.fileName}</span>
          <span className="text-xs text-muted-foreground">
            {field.contentType && <span>{field.contentType} &middot; </span>}
            {formatBytes(field.size)}
          </span>
        </div>
      ) : (
        <span className="text-foreground">
          {(field.value?.length ?? 0) > 500
            ? field.value!.slice(0, 500) + "..."
            : field.value ?? ""}
        </span>
      )}
    </td>
    <td className="px-3 py-2 text-sm text-muted-foreground text-right align-top whitespace-nowrap">
      {field.isFile ? "File" : "Text"}
    </td>
  </tr>
);

export const MultipartPreview = ({ data, contentType }: MultipartPreviewProps) => {
  const fields = useMemo<MultipartField[]>(() => {
    if (!data || data.length === 0) return [];
    const boundary = extractBoundary(contentType);
    if (!boundary) return [];
    return parseMultipartFormData(data, boundary);
  }, [data, contentType]);

  if (fields.length === 0) {
    return (
      <div className="h-[calc(100vh-300px)] border rounded-md flex items-center justify-center text-muted-foreground text-sm">
        No multipart fields found
      </div>
    );
  }

  return (
    <div className="h-[calc(100vh-300px)] border rounded-md overflow-auto">
      {/* 헤더 */}
      <div className="flex items-center gap-3 p-3 border-b bg-muted/30 text-xs text-muted-foreground">
        <span className="font-medium text-foreground">multipart/form-data</span>
        <span>{fields.length} fields</span>
        <span>
          {fields.filter((f) => f.isFile).length} files
        </span>
      </div>

      {/* 테이블 */}
      <table className="w-full">
        <thead>
          <tr className="border-b bg-muted/20">
            <th className="px-3 py-1.5 text-left text-xs font-medium text-muted-foreground w-1/4">
              Field
            </th>
            <th className="px-3 py-1.5 text-left text-xs font-medium text-muted-foreground">
              Value
            </th>
            <th className="px-3 py-1.5 text-right text-xs font-medium text-muted-foreground w-16">
              Type
            </th>
          </tr>
        </thead>
        <tbody>
          {fields.map((field, i) => (
            <FieldRow key={`${field.name}-${i}`} field={field} />
          ))}
        </tbody>
      </table>
    </div>
  );
};
