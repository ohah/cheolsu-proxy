import { useState, useEffect } from "react";
import { Download, FileArchive, FileText, FileType, HardDrive, Eye, EyeOff } from "lucide-react";
import { readFile, BaseDirectory } from "@tauri-apps/plugin-fs";

import type { DataType } from "@/entities/proxy/model/data-type";
import {
  dataTypeToDisplayName,
  dataTypeToIcon,
} from "@/entities/proxy/model/data-type";
import { Button } from "@/shared/ui";
import { formatBytes } from "@/widgets/network-table/lib/utils";

interface BinaryPreviewProps {
  data?: Uint8Array | null;
  dataType: DataType;
  bodySize: number;
  fileName: string;
  fileExtension: string;
  mimeType: string;
  filePath?: string;
  className?: string;
}

const HEX_BYTES_PER_ROW = 16;
const HEX_INITIAL_ROWS = 32;
const HEX_MAX_ROWS = 256;

function formatHexDump(data: Uint8Array, maxRows: number): string {
  const rows = Math.min(Math.ceil(data.length / HEX_BYTES_PER_ROW), maxRows);
  const lines: string[] = [];

  for (let row = 0; row < rows; row++) {
    const offset = row * HEX_BYTES_PER_ROW;
    const offsetStr = offset.toString(16).padStart(8, "0");

    const hexParts: string[] = [];
    const asciiParts: string[] = [];

    for (let col = 0; col < HEX_BYTES_PER_ROW; col++) {
      const idx = offset + col;
      if (idx < data.length) {
        hexParts.push(data[idx].toString(16).padStart(2, "0"));
        const ch = data[idx];
        asciiParts.push(ch >= 0x20 && ch <= 0x7e ? String.fromCharCode(ch) : ".");
      } else {
        hexParts.push("  ");
        asciiParts.push(" ");
      }
    }

    // 8바이트 단위로 구분
    const hexLeft = hexParts.slice(0, 8).join(" ");
    const hexRight = hexParts.slice(8).join(" ");
    const ascii = asciiParts.join("");

    lines.push(`${offsetStr}  ${hexLeft}  ${hexRight}  |${ascii}|`);
  }

  if (data.length > rows * HEX_BYTES_PER_ROW) {
    lines.push(`... (${formatBytes(data.length - rows * HEX_BYTES_PER_ROW)} more)`);
  }

  return lines.join("\n");
}

const FileTypeIcon = ({ dataType }: { dataType: DataType }) => {
  switch (dataType) {
    case "Document":
      return <FileText className="w-10 h-10 text-red-500" />;
    case "Archive":
      return <FileArchive className="w-10 h-10 text-amber-500" />;
    case "Binary":
      return <HardDrive className="w-10 h-10 text-slate-500" />;
    default:
      return <FileType className="w-10 h-10 text-muted-foreground" />;
  }
};

export const BinaryPreview = ({
  data,
  dataType,
  bodySize,
  fileName,
  fileExtension,
  mimeType,
  filePath,
  className,
}: BinaryPreviewProps) => {
  const [showHex, setShowHex] = useState(false);
  const [hexMaxRows, setHexMaxRows] = useState(HEX_INITIAL_ROWS);
  const [fileData, setFileData] = useState<Uint8Array | null>(null);
  const [pdfUrl, setPdfUrl] = useState<string>("");
  const [showPdf, setShowPdf] = useState(false);
  const [loading, setLoading] = useState(false);

  // 파일 데이터 로드 (hex dump 또는 PDF 미리보기용)
  const loadFileData = async (): Promise<Uint8Array | null> => {
    if (data && data.length > 0) return data;
    if (fileData) return fileData;
    if (!filePath) return null;

    setLoading(true);
    try {
      const rawData = await readFile(filePath, { baseDir: BaseDirectory.Cache });
      const result = new Uint8Array(rawData);
      setFileData(result);
      return result;
    } catch (err) {
      console.error("파일 읽기 실패:", err);
      return null;
    } finally {
      setLoading(false);
    }
  };

  // PDF URL 생성
  useEffect(() => {
    if (!showPdf || dataType !== "Document") return;

    const createPdfUrl = async () => {
      const loaded = await loadFileData();
      if (!loaded) return;
      const blob = new Blob([loaded.buffer as ArrayBuffer], { type: "application/pdf" });
      const url = URL.createObjectURL(blob);
      setPdfUrl(url);
    };

    createPdfUrl();

    return () => {
      if (pdfUrl) URL.revokeObjectURL(pdfUrl);
    };
  }, [showPdf]);

  const handleDownload = async () => {
    const loaded = await loadFileData();
    if (!loaded) return;

    const blob = new Blob([loaded.buffer as ArrayBuffer], { type: mimeType || "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const handleToggleHex = async () => {
    if (!showHex && !data && !fileData) {
      await loadFileData();
    }
    setShowHex(!showHex);
    setHexMaxRows(HEX_INITIAL_ROWS);
  };

  const actualData = data || fileData;

  return (
    <div className={`h-[calc(100vh-300px)] border rounded-md overflow-auto ${className || ""}`}>
      <div className="flex flex-col h-full">
        {/* 파일 정보 카드 */}
        <div className="flex items-center gap-4 p-6 border-b">
          <FileTypeIcon dataType={dataType} />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm truncate">{fileName}</span>
              {fileExtension && (
                <span className="px-1.5 py-0.5 text-xs font-mono bg-muted rounded uppercase flex-shrink-0">
                  .{fileExtension}
                </span>
              )}
            </div>
            <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
              <span>{dataTypeToDisplayName(dataType)}</span>
              <span>{formatBytes(bodySize)}</span>
              {mimeType && <span className="font-mono">{mimeType}</span>}
            </div>
          </div>
          <div className="flex items-center gap-1 flex-shrink-0">
            {dataType === "Document" && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowPdf(!showPdf)}
                title="PDF 미리보기"
              >
                {showPdf ? <EyeOff className="w-4 h-4 mr-1" /> : <Eye className="w-4 h-4 mr-1" />}
                {showPdf ? "닫기" : "미리보기"}
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              onClick={handleToggleHex}
              disabled={loading}
              title="Hex Dump 보기"
            >
              {showHex ? "Hex 닫기" : "Hex 보기"}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleDownload}
              disabled={loading}
              title="파일 다운로드"
            >
              <Download className="w-4 h-4 mr-1" />
              다운로드
            </Button>
          </div>
        </div>

        {/* PDF 미리보기 */}
        {showPdf && dataType === "Document" && (
          <div className="flex-1 min-h-0">
            {pdfUrl ? (
              <embed
                src={pdfUrl}
                type="application/pdf"
                className="w-full h-full"
              />
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                PDF 로딩 중...
              </div>
            )}
          </div>
        )}

        {/* Hex Dump */}
        {showHex && !showPdf && (
          <div className="flex-1 min-h-0 overflow-auto">
            {loading ? (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                데이터 로딩 중...
              </div>
            ) : actualData ? (
              <div className="p-4">
                <pre className="font-mono text-xs leading-5 text-foreground whitespace-pre">
                  {formatHexDump(actualData, hexMaxRows)}
                </pre>
                {actualData.length > hexMaxRows * HEX_BYTES_PER_ROW && hexMaxRows < HEX_MAX_ROWS && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="mt-2"
                    onClick={() => setHexMaxRows(Math.min(hexMaxRows * 2, HEX_MAX_ROWS))}
                  >
                    더 보기
                  </Button>
                )}
              </div>
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                데이터를 불러올 수 없습니다
              </div>
            )}
          </div>
        )}

        {/* 기본 상태: 아이콘과 안내 */}
        {!showHex && !showPdf && (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <span className="text-4xl">{dataTypeToIcon(dataType)}</span>
              <p className="mt-2 text-sm">
                이 파일은 바이너리 형식입니다
              </p>
              <p className="text-xs mt-1">
                Hex 보기 또는 다운로드를 사용하세요
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
