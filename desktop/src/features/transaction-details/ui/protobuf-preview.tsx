import { useState, useMemo, useEffect } from "react";
import { ChevronRight, ChevronDown, AlertTriangle, Download } from "lucide-react";
import { readFile, BaseDirectory } from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";

import type { DataType } from "@/entities/proxy";
import { Button } from "@/shared/ui";
import { formatBytes } from "@/shared/lib";
import {
  decodeProtobuf,
  wireTypeName,
  bytesToHex,
  type ProtobufField,
  type ProtobufValue,
} from "../lib/protobuf-decoder";

interface ProtobufPreviewProps {
  data?: Uint8Array | null;
  dataType: DataType;
  bodySize: number;
  contentType: string;
  filePath?: string;
  /** gRPC 서비스명 (예: "package.ServiceName") */
  grpcService?: string;
  /** gRPC 메서드명 (예: "MethodName") */
  grpcMethod?: string;
  /** 요청인지 응답인지 */
  isRequest?: boolean;
}

const FieldNode = ({ field, depth = 0 }: { field: ProtobufField; depth?: number }) => {
  const [expanded, setExpanded] = useState(depth < 3);
  const isNested = field.value.type === "message";

  return (
    <div style={{ paddingLeft: depth > 0 ? 16 : 0 }}>
      <div
        className="flex items-center gap-1.5 py-0.5 hover:bg-muted/50 rounded px-1 cursor-default text-sm font-mono"
        onClick={isNested ? () => setExpanded(!expanded) : undefined}
      >
        {isNested ? (
          <button className="w-4 h-4 flex items-center justify-center flex-shrink-0 text-muted-foreground">
            {expanded ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
          </button>
        ) : (
          <span className="w-4 h-4 flex-shrink-0" />
        )}

        <span className="text-blue-500 dark:text-blue-400">{field.fieldNumber}</span>

        <span className="px-1 py-0 text-[10px] rounded bg-muted text-muted-foreground">
          {wireTypeName(field.wireType)}
        </span>

        <FieldValue value={field.value} />
      </div>

      {isNested && expanded && field.value.type === "message" && (
        <div className="border-l border-muted ml-2.5">
          {field.value.fields.map((child, i) => (
            <FieldNode key={`${child.fieldNumber}-${i}`} field={child} depth={depth + 1} />
          ))}
        </div>
      )}
    </div>
  );
};

const FieldValue = ({ value }: { value: ProtobufValue }) => {
  switch (value.type) {
    case "varint":
      return (
        <span className="text-foreground">
          <span className="text-green-600 dark:text-green-400">{value.raw.toString()}</span>
          {value.signed !== value.raw && (
            <span className="text-muted-foreground ml-1.5 text-xs">
              (signed: {value.signed.toString()})
            </span>
          )}
          {value.raw <= 1n && (
            <span className="text-muted-foreground ml-1.5 text-xs">
              (bool: {value.bool.toString()})
            </span>
          )}
        </span>
      );
    case "fixed64":
      return (
        <span className="text-foreground">
          <span className="text-green-600 dark:text-green-400">{value.raw.toString()}</span>
          {Number.isFinite(value.asDouble) && value.asDouble !== 0 && (
            <span className="text-muted-foreground ml-1.5 text-xs">(double: {value.asDouble})</span>
          )}
        </span>
      );
    case "fixed32":
      return (
        <span className="text-foreground">
          <span className="text-green-600 dark:text-green-400">{value.raw}</span>
          {Number.isFinite(value.asFloat) && value.asFloat !== 0 && (
            <span className="text-muted-foreground ml-1.5 text-xs">
              (float: {value.asFloat.toFixed(6)})
            </span>
          )}
        </span>
      );
    case "string":
      return (
        <span className="text-amber-600 dark:text-amber-400">
          &quot;{value.value.length > 200 ? value.value.slice(0, 200) + "..." : value.value}&quot;
        </span>
      );
    case "bytes":
      return (
        <span className="text-muted-foreground text-xs">
          [{formatBytes(value.value.length)}] {bytesToHex(value.value, 16)}
        </span>
      );
    case "message":
      return (
        <span className="text-muted-foreground text-xs">{`{${value.fields.length} fields}`}</span>
      );
    default:
      return null;
  }
};

const INITIAL_FIELDS_LIMIT = 200;

export const ProtobufPreview = ({
  data,
  bodySize,
  contentType,
  filePath,
  grpcService,
  grpcMethod,
  isRequest,
}: ProtobufPreviewProps) => {
  const [fileData, setFileData] = useState<Uint8Array | null>(null);
  const [loading, setLoading] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [protoDecoded, setProtoDecoded] = useState<Record<string, unknown> | null>(null);

  const actualData = data || fileData;

  const loadFileData = async () => {
    if (data && data.length > 0) return;
    if (fileData) return;
    if (!filePath) return;

    setLoading(true);
    try {
      const appCachePath = filePath.startsWith("com.cheolsu-proxy/")
        ? filePath.slice("com.cheolsu-proxy/".length)
        : filePath;
      const rawData = await readFile(appCachePath, { baseDir: BaseDirectory.AppCache });
      setFileData(new Uint8Array(rawData));
    } finally {
      setLoading(false);
    }
  };

  // 자동 로드
  useEffect(() => {
    if (!actualData && filePath && !loading) {
      loadFileData();
    }
  }, [filePath, actualData, loading]);

  // .proto 파일이 등록되어 있으면 Tauri를 통해 디코딩 시도
  useEffect(() => {
    if (!actualData || actualData.length === 0 || !grpcService || !grpcMethod) {
      setProtoDecoded(null);
      return;
    }
    invoke<Record<string, unknown> | null>("decode_grpc_message", {
      service: grpcService,
      method: grpcMethod,
      data: Array.from(actualData),
      isRequest: isRequest ?? false,
    })
      .then((result) => setProtoDecoded(result ?? null))
      .catch(() => setProtoDecoded(null));
  }, [actualData, grpcService, grpcMethod, isRequest]);

  const decoded = useMemo(() => {
    if (!actualData || actualData.length === 0) return null;
    try {
      return decodeProtobuf(actualData, contentType);
    } catch (e) {
      return { error: e instanceof Error ? e.message : "decode failed" };
    }
  }, [actualData, contentType]);

  const handleDownload = () => {
    if (!actualData) return;
    const blob = new Blob([actualData.buffer as ArrayBuffer], {
      type: contentType || "application/x-protobuf",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "protobuf-data.pb";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  if (loading) {
    return (
      <div className="h-[calc(100vh-300px)] border rounded-md flex items-center justify-center text-muted-foreground">
        Loading...
      </div>
    );
  }

  if (!actualData || !decoded) {
    return (
      <div className="h-[calc(100vh-300px)] border rounded-md flex items-center justify-center text-muted-foreground">
        No data
      </div>
    );
  }

  if ("error" in decoded) {
    return (
      <div className="h-[calc(100vh-300px)] border rounded-md overflow-auto">
        <div className="flex items-center gap-2 p-4 border-b text-destructive">
          <AlertTriangle className="w-4 h-4" />
          <span className="text-sm">Protobuf 디코딩 실패: {decoded.error}</span>
        </div>
        <div className="p-4">
          <pre className="font-mono text-xs text-muted-foreground">
            {bytesToHex(actualData, 256)}
          </pre>
        </div>
      </div>
    );
  }

  return (
    <div className="h-[calc(100vh-300px)] border rounded-md overflow-auto">
      {/* 헤더 */}
      <div className="flex items-center justify-between p-3 border-b bg-muted/30">
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          <span className="font-medium text-foreground">Protobuf</span>
          {decoded.isGrpc && (
            <span className="px-1.5 py-0.5 rounded bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300 text-[10px] font-medium">
              gRPC
            </span>
          )}
          <span>{formatBytes(bodySize)}</span>
          <span>{decoded.fields.length} fields</span>
          {decoded.messageCount > 1 && <span>{decoded.messageCount} messages</span>}
        </div>
        <Button variant="ghost" size="sm" onClick={handleDownload}>
          <Download className="w-3.5 h-3.5 mr-1" />
          Download
        </Button>
      </div>

      {/* Proto 디코딩 결과 (필드명 포함 JSON) */}
      {protoDecoded && (
        <div className="border-b">
          <div className="px-3 py-1.5 bg-green-50 dark:bg-green-950 text-green-700 dark:text-green-300 text-xs font-medium flex items-center gap-1.5">
            <span className="px-1 py-0.5 rounded bg-green-100 dark:bg-green-900 text-[10px]">
              .proto
            </span>
            Decoded with registered proto file
          </div>
          <pre className="p-3 text-sm font-mono overflow-auto max-h-96">
            {JSON.stringify(protoDecoded, null, 2)}
          </pre>
        </div>
      )}

      {/* Wire format 트리 뷰 */}
      <div className="p-3">
        {(showAll ? decoded.fields : decoded.fields.slice(0, INITIAL_FIELDS_LIMIT)).map(
          (field, i) => (
            <FieldNode key={`${field.fieldNumber}-${i}`} field={field} />
          ),
        )}
        {!showAll && decoded.fields.length > INITIAL_FIELDS_LIMIT && (
          <Button variant="ghost" size="sm" className="mt-2" onClick={() => setShowAll(true)}>
            Show all ({decoded.fields.length - INITIAL_FIELDS_LIMIT} more fields)
          </Button>
        )}
      </div>
    </div>
  );
};
