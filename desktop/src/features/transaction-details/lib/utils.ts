import { type DataType, isTextBasedDataType, isBinaryDataType } from "@/entities/proxy";
import type { HttpTransaction } from "@/entities/proxy";
import type { ReplayRequestParams } from "@/shared/api/proxy";
import { sanitizeHopByHopHeaders } from "@/shared/lib/http-headers";

/**
 * Uint8Array를 문자열로 변환 (UTF-8 디코딩)
 * 러스트에서 이미 GZIP 압축 해제와 데이터 타입 감지를 완료했으므로 단순한 UTF-8 디코딩만 수행
 */
export const uint8ArrayToString = (data: Uint8Array | number[], dataType: DataType): string => {
  if (!data || data.length === 0) {
    return "";
  }

  try {
    // 일반 배열인 경우 Uint8Array로 변환
    const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

    // 러스트에서 이미 처리된 데이터이므로 단순한 UTF-8 디코딩
    const decoder = new TextDecoder("utf-8", { fatal: false });
    return decoder.decode(uint8Array);
  } catch (error) {
    console.error("UTF-8 디코딩 실패:", error);
    return `디코딩 실패 (${dataType})`;
  }
};

/**
 * HTML 엔티티 디코딩
 */
export const decodeHtmlEntities = (text: string): string => {
  const textarea = document.createElement("textarea");
  textarea.innerHTML = text;
  return textarea.value;
};

/**
 * Uint8Array를 Base64 문자열로 변환
 */
export const uint8ArrayToBase64 = (data: Uint8Array | number[]): string => {
  if (!data || data.length === 0) {
    return "";
  }

  try {
    // 일반 배열인 경우 Uint8Array로 변환
    const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

    // Uint8Array를 문자열로 변환한 후 Base64 인코딩 (chunked approach to avoid O(n²))
    const chunks: string[] = [];
    const chunkSize = 8192;
    for (let i = 0; i < uint8Array.length; i += chunkSize) {
      chunks.push(String.fromCharCode(...uint8Array.subarray(i, i + chunkSize)));
    }
    return btoa(chunks.join(""));
  } catch (error) {
    console.error("Base64 인코딩 실패:", error);
    return "";
  }
};

/**
 * 이미지 데이터를 Data URL로 변환
 */
export const createImageDataUrl = (data: Uint8Array | number[], dataType: DataType): string => {
  if (dataType !== "Image") {
    return "";
  }

  const base64 = uint8ArrayToBase64(data);
  if (!base64) {
    return "";
  }

  // MIME 타입 결정
  let mimeType = "image/png"; // 기본값
  if (data.length >= 2) {
    const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

    // PNG 시그니처
    if (
      uint8Array.length >= 8 &&
      uint8Array[0] === 0x89 &&
      uint8Array[1] === 0x50 &&
      uint8Array[2] === 0x4e &&
      uint8Array[3] === 0x47
    ) {
      mimeType = "image/png";
    }
    // JPEG 시그니처
    else if (uint8Array[0] === 0xff && uint8Array[1] === 0xd8) {
      mimeType = "image/jpeg";
    }
    // GIF 시그니처
    else if (
      uint8Array.length >= 6 &&
      ((uint8Array[0] === 0x47 &&
        uint8Array[1] === 0x49 &&
        uint8Array[2] === 0x46 &&
        uint8Array[3] === 0x38 &&
        uint8Array[4] === 0x37 &&
        uint8Array[5] === 0x61) ||
        (uint8Array[0] === 0x47 &&
          uint8Array[1] === 0x49 &&
          uint8Array[2] === 0x46 &&
          uint8Array[3] === 0x38 &&
          uint8Array[4] === 0x39 &&
          uint8Array[5] === 0x61))
    ) {
      mimeType = "image/gif";
    }
    // WebP 시그니처
    else if (
      uint8Array.length >= 12 &&
      uint8Array[0] === 0x52 &&
      uint8Array[1] === 0x49 &&
      uint8Array[2] === 0x46 &&
      uint8Array[3] === 0x46 &&
      uint8Array[8] === 0x57 &&
      uint8Array[9] === 0x45 &&
      uint8Array[10] === 0x42 &&
      uint8Array[11] === 0x50
    ) {
      mimeType = "image/webp";
    }
    // SVG는 텍스트 기반이므로 별도 처리
    else if (
      uint8Array.length >= 4 &&
      uint8Array[0] === 0x3c &&
      uint8Array[1] === 0x73 &&
      uint8Array[2] === 0x76 &&
      uint8Array[3] === 0x67
    ) {
      mimeType = "image/svg+xml";
    }
  }

  return `data:${mimeType};base64,${base64}`;
};

/**
 * 요청/응답 본문을 포맷팅된 문자열로 변환
 * 러스트에서 이미 데이터 타입 감지와 압축 해제를 완료했으므로 단순한 포맷팅만 수행
 */
export const formatBodyContent = (
  body: Uint8Array,
  dataType: DataType,
  bodyJson?: unknown,
): string => {
  if (dataType === "Empty") {
    return "";
  }

  // GraphQL 타입이면 query 필드를 추출하여 포맷팅
  if (dataType === "GraphQL" && bodyJson) {
    return formatGraphQLBody(bodyJson);
  }

  // JSON 타입이고 body_json이 있으면 바로 포맷팅
  if (dataType === "Json" && bodyJson) {
    return JSON.stringify(bodyJson, null, 2);
  }

  if (isTextBasedDataType(dataType)) {
    const text = uint8ArrayToString(body, dataType);

    // JSON 타입인 경우 포맷팅 시도 (fallback)
    if (dataType === "Json") {
      try {
        const parsed = JSON.parse(text);
        return JSON.stringify(parsed, null, 2);
      } catch (error) {
        console.warn("JSON 파싱 실패, 원본 텍스트 반환:", error);
        return decodeHtmlEntities(text);
      }
    }

    return decodeHtmlEntities(text);
  }

  if (isBinaryDataType(dataType)) {
    return `[${dataType} - ${body.length} bytes]`;
  }

  return uint8ArrayToString(body, dataType);
};

/**
 * 요청/응답 본문을 표시용으로 변환 (Monaco Editor용)
 */
export const getBodyForDisplay = (
  body: Uint8Array,
  dataType: DataType,
  bodyJson?: unknown,
): string => {
  if (dataType === "Empty") {
    return "";
  }

  if (isTextBasedDataType(dataType)) {
    return formatBodyContent(body, dataType, bodyJson);
  }

  if (isBinaryDataType(dataType)) {
    return `// ${dataType} 파일 (${body.length} bytes)\n// 이 파일은 바이너리 형식이므로 텍스트로 표시할 수 없습니다.`;
  }

  return formatBodyContent(body, dataType, bodyJson);
};

// generateCurlCommand는 shared/lib/curl.ts로 이동됨
export { generateCurlCommand } from "@/shared/lib/curl";

/**
 * URL 경로에서 파일 이름을 추출
 */
export const getFileNameFromUrl = (uri: string): string | null => {
  try {
    const url = new URL(uri);
    const pathSegments = url.pathname.split("/").filter(Boolean);
    const lastSegment = pathSegments[pathSegments.length - 1];
    if (lastSegment && lastSegment.includes(".")) {
      return decodeURIComponent(lastSegment);
    }
    return null;
  } catch {
    return null;
  }
};

/**
 * Content-Disposition 헤더에서 파일 이름을 추출
 */
export const getFileNameFromContentDisposition = (
  headers: Record<string, string>,
): string | null => {
  const disposition = headers["content-disposition"];
  if (!disposition) return null;

  const utf8Match = disposition.match(/filename\*=(?:UTF-8''|utf-8'')([^;]+)/i);
  if (utf8Match) {
    try {
      return decodeURIComponent(utf8Match[1]);
    } catch {
      return utf8Match[1];
    }
  }

  const quotedMatch = disposition.match(/filename="([^"]+)"/i);
  if (quotedMatch) return quotedMatch[1];

  const plainMatch = disposition.match(/filename=([^;\s]+)/i);
  if (plainMatch) return plainMatch[1];

  return null;
};

/**
 * file_path에서 확장자를 추출
 */
export const getExtensionFromPath = (path: string): string => {
  const fileName = path.split("/").pop() || "";
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex > 0) {
    return fileName.substring(dotIndex + 1).toLowerCase();
  }
  return "";
};

/**
 * 바이너리 데이터의 파일 정보를 추출
 */
export const extractBinaryFileInfo = (
  uri: string,
  headers: Record<string, string>,
  dataType: DataType,
  filePath?: string,
  bodySize?: number,
): { fileName: string; fileExtension: string; mimeType: string } => {
  const contentType = headers["content-type"] || "";
  const mimeType = contentType.split(";")[0].trim();

  let fileExtension = "";
  if (filePath) {
    fileExtension = getExtensionFromPath(filePath);
  }
  if (!fileExtension && mimeType) {
    const mimeToExt: Record<string, string> = {
      "application/pdf": "pdf",
      "application/zip": "zip",
      "application/x-rar-compressed": "rar",
      "application/x-7z-compressed": "7z",
      "application/gzip": "gz",
      "application/x-tar": "tar",
      "application/octet-stream": "bin",
      "application/wasm": "wasm",
      "font/woff": "woff",
      "font/woff2": "woff2",
      "font/ttf": "ttf",
      "font/otf": "otf",
    };
    fileExtension = mimeToExt[mimeType] || "";
  }
  if (!fileExtension) {
    const typeToExt: Record<string, string> = {
      Document: "pdf",
      Archive: "zip",
      Binary: "bin",
    };
    fileExtension = typeToExt[dataType] || "bin";
  }

  let fileName = getFileNameFromContentDisposition(headers);
  if (!fileName) {
    fileName = getFileNameFromUrl(uri);
  }
  if (!fileName) {
    const size = bodySize ? `_${bodySize}` : "";
    fileName = `file${size}.${fileExtension}`;
  }

  return { fileName, fileExtension, mimeType };
};

/**
 * GraphQL body를 보기 좋게 포맷팅
 */
interface GraphQLBody {
  operationName?: string;
  query?: string;
  variables?: Record<string, unknown>;
  extensions?: Record<string, unknown>;
}

const isGraphQLBody = (value: unknown): value is GraphQLBody => {
  return (
    typeof value === "object" && value !== null && ("query" in value || "operationName" in value)
  );
};

const formatGraphQLBody = (bodyJson: unknown): string => {
  if (!isGraphQLBody(bodyJson)) {
    return String(bodyJson);
  }

  const parts: string[] = [];

  if (bodyJson.operationName) {
    parts.push(`# Operation: ${bodyJson.operationName}`);
  }

  if (typeof bodyJson.query === "string") {
    parts.push(bodyJson.query.trim());
  }

  if (bodyJson.variables && Object.keys(bodyJson.variables).length > 0) {
    parts.push(`\n# Variables\n${JSON.stringify(bodyJson.variables, null, 2)}`);
  }

  if (bodyJson.extensions && Object.keys(bodyJson.extensions).length > 0) {
    parts.push(`\n# Extensions\n${JSON.stringify(bodyJson.extensions, null, 2)}`);
  }

  return parts.join("\n");
};

/**
 * HttpTransaction을 ReplayRequestParams로 변환
 * body_json 포함 처리
 */
export function transactionToReplayParams(
  transaction: HttpTransaction,
): ReplayRequestParams | null {
  const { request } = transaction;
  if (!request) return null;

  const headers = sanitizeHopByHopHeaders(request.headers);

  let body: string | undefined;
  if (request.body && request.data_type && isTextBasedDataType(request.data_type)) {
    body = uint8ArrayToString(request.body, request.data_type);
  } else if (request.body_json) {
    body =
      typeof request.body_json === "string" ? request.body_json : JSON.stringify(request.body_json);
  }

  return {
    method: request.method,
    url: request.uri,
    headers,
    body,
  };
}

// Re-export data type utilities for convenience
export {
  dataTypeToMonacoLanguage,
  dataTypeToMimeType,
  dataTypeToDisplayName,
  dataTypeToIcon,
  isTextBasedDataType,
  isImageDataType,
  isVideoDataType,
  isAudioDataType,
  isCompressedDataType,
  isBinaryDataType,
} from "@/entities/proxy";
