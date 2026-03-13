import { Copy, FileText, Loader2 } from "lucide-react";
import { writeImage, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useTheme } from "next-themes";

import type { HttpTransaction } from "@/entities/proxy";

import { Button, Card, CardContent, CardHeader } from "@/shared/ui";
import { Editor } from "@monaco-editor/react";

import { getBodyForDisplay, createImageDataUrl, extractBinaryFileInfo } from "../lib/utils";
import { isMultipartFormData, isUrlencoded } from "../lib/form-data-parser";
import {
  dataTypeToMonacoLanguage,
  isImageDataType,
  isMediaDataType,
  isBinaryDataType,
  isProtobufDataType,
} from "@/entities/proxy/model/data-type";
import { toast } from "sonner";
import { MediaPreview } from "./media-preview";
import { BinaryPreview } from "./binary-preview";
import { ProtobufPreview } from "./protobuf-preview";
import { MultipartPreview } from "./multipart-preview";
import { UrlencodedPreview } from "./urlencoded-preview";
import { useBodyFile } from "@/hooks/use-body-file";

interface TransactionBodyProps {
  transaction: HttpTransaction;
  compact?: boolean;
}

export const TransactionBody = ({ transaction, compact }: TransactionBodyProps) => {
  const { resolvedTheme } = useTheme();
  const { request } = transaction;
  const heightClass = compact ? "h-full" : "h-[calc(100vh-300px)]";

  if (!request) return null;

  // 바이너리 파일 정보 추출
  const isNonMediaBinary =
    isBinaryDataType(request.data_type) && !isMediaDataType(request.data_type);
  const binaryFileInfo = isNonMediaBinary
    ? extractBinaryFileInfo(
        request.uri,
        request.headers,
        request.data_type,
        request.file_path,
        request.body_size,
      )
    : null;

  // 파일에서 body를 읽어오는 훅
  const {
    body: fileBody,
    loading: fileLoading,
    error: fileError,
  } = useBodyFile(request.file_path, !!request.file_path);

  // 실제 사용할 body 데이터 (파일이 있으면 파일에서 읽어온 것, 없으면 메모리의 것)
  const actualBody = request.file_path ? fileBody : request.body || null;

  // Content-Type 헤더에서 MIME 타입 추출
  const contentTypeHeader = request.headers["content-type"] || "";
  const getMimeType = () => {
    return contentTypeHeader;
  };

  // form-data 타입 판별
  const isMultipart = isMultipartFormData(contentTypeHeader);
  const isUrlencodedBody = isUrlencoded(contentTypeHeader);

  const getRequestText = () => {
    // 파일이 있고 로딩 중이면 로딩 메시지 표시
    if (request.file_path && fileLoading) {
      return "파일을 로딩 중입니다...";
    }

    // 파일이 있고 에러가 발생했으면 에러 메시지 표시
    if (request.file_path && fileError) {
      return `파일 로딩 실패: ${fileError}`;
    }

    if (!actualBody || actualBody.length === 0) {
      return "";
    }
    return getBodyForDisplay(actualBody, request.data_type, request.body_json);
  };

  const requestText = getRequestText();

  const handleCopy = async () => {
    if (actualBody && actualBody.length > 0 && isImageDataType(request.data_type)) {
      try {
        // Tauri 클립보드 매니저를 사용하여 이미지 복사
        await writeImage(actualBody);
        console.log("Image copied successfully via Tauri clipboard manager");
        toast.success("Image copied to clipboard");
      } catch (error) {
        console.error("Failed to copy image via Tauri clipboard manager:", error);
        toast.error("Failed to copy image");

        // Tauri 클립보드 실패 시 다운로드로 fallback
        try {
          console.log("Attempting fallback download...");
          const dataUrl = createImageDataUrl(actualBody, request.data_type);
          if (dataUrl) {
            const link = document.createElement("a");
            link.href = dataUrl;
            link.download = `image.${request.data_type}`;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            toast.success("Image downloaded (clipboard failed)");
            console.log("Image downloaded as fallback");
          } else {
            console.error("Failed to generate dataUrl for fallback download");
            toast.error("Failed to copy or download image");
          }
        } catch (fallbackError) {
          console.error("Fallback download failed:", fallbackError);
          toast.error("Failed to copy or download image");
        }
      }
    } else {
      try {
        // Tauri 클립보드 매니저를 사용하여 텍스트 복사
        await writeText(requestText);
        toast.success("Request body copied to clipboard");
      } catch (error) {
        console.error("Failed to copy text:", error);
        toast.error("Failed to copy to clipboard");
      }
    }
  };

  return (
    <Card className="gap-0 flex flex-col min-h-0 flex-1">
      <CardHeader className="flex-shrink-0">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {request.file_path && (
              <div className="flex items-center gap-1 text-sm text-muted-foreground">
                <FileText className="w-4 h-4" />
                {fileLoading && <Loader2 className="w-3 h-3 animate-spin" />}
                {fileError && <span className="text-destructive">오류</span>}
              </div>
            )}
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopy}
            title="요청 Body 내용을 클립보드에 복사"
          >
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 p-0 min-h-0">
        {actualBody &&
        actualBody.length > 0 &&
        isMediaDataType(request.data_type) &&
        !fileLoading &&
        !fileError ? (
          <div className={`${heightClass} border rounded-md overflow-auto p-4`}>
            <MediaPreview
              data={request.file_path ? undefined : actualBody}
              dataType={request.data_type}
              className="h-full"
              mimeType={getMimeType()}
              filePath={request.file_path}
            />
          </div>
        ) : isProtobufDataType(request.data_type) && !fileLoading && !fileError ? (
          <ProtobufPreview
            data={actualBody}
            dataType={request.data_type}
            bodySize={request.body_size}
            contentType={request.headers["content-type"] || ""}
            filePath={request.file_path}
          />
        ) : isMultipart && actualBody && actualBody.length > 0 && !fileLoading && !fileError ? (
          <MultipartPreview data={actualBody} contentType={contentTypeHeader} />
        ) : isUrlencodedBody &&
          actualBody &&
          actualBody.length > 0 &&
          !fileLoading &&
          !fileError ? (
          <UrlencodedPreview data={actualBody} />
        ) : isNonMediaBinary && binaryFileInfo && !fileLoading && !fileError ? (
          <BinaryPreview
            data={actualBody}
            dataType={request.data_type}
            bodySize={request.body_size}
            fileName={binaryFileInfo.fileName}
            fileExtension={binaryFileInfo.fileExtension}
            mimeType={binaryFileInfo.mimeType}
            filePath={request.file_path}
          />
        ) : (
          <div className={`${heightClass} border rounded-md overflow-hidden`}>
            <Editor
              height={compact ? "100%" : "calc(100vh - 300px)"}
              language={dataTypeToMonacoLanguage(request.data_type)}
              value={requestText}
              theme={resolvedTheme === "dark" ? "vs-dark" : "vs"}
              options={{
                readOnly: true,
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                fontSize: 12,
                lineNumbers: "on",
                wordWrap: "on",
                automaticLayout: true,
                padding: { top: 8, bottom: 8 },
                scrollbar: {
                  vertical: "auto",
                  horizontal: "auto",
                },
              }}
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
};
