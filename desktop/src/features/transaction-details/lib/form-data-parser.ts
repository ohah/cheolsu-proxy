/**
 * multipart/form-data 및 application/x-www-form-urlencoded 파싱 유틸리티
 */

// ─── Types ───────────────────────────────────────────────────────────

export interface MultipartField {
  /** 필드 이름 (Content-Disposition의 name) */
  name: string;
  /** 텍스트 필드의 값 (파일이 아닌 경우) */
  value?: string;
  /** 파일명 (파일 업로드인 경우) */
  fileName?: string;
  /** 파트의 Content-Type */
  contentType?: string;
  /** 파일인지 여부 */
  isFile: boolean;
  /** 파트의 바이트 크기 */
  size: number;
}

export interface UrlencodedField {
  /** 키 (URL 디코딩 완료) */
  key: string;
  /** 값 (URL 디코딩 완료) */
  value: string;
}

// ─── Boundary 추출 ───────────────────────────────────────────────────

/**
 * Content-Type 헤더에서 boundary를 추출한다.
 * 예: "multipart/form-data; boundary=----WebKitFormBoundary7MA4YWxkTrZu0gW"
 */
export const extractBoundary = (contentType: string): string | null => {
  if (!contentType) return null;

  const match = contentType.match(/boundary=(?:"([^"]+)"|([^\s;]+))/i);
  if (!match) return null;

  return match[1] || match[2] || null;
};

// ─── Content-Type 판별 ───────────────────────────────────────────────

/**
 * Content-Type이 multipart/form-data인지 확인
 */
export const isMultipartFormData = (contentType: string): boolean => {
  if (!contentType) return false;
  return contentType.toLowerCase().startsWith("multipart/form-data");
};

/**
 * Content-Type이 application/x-www-form-urlencoded인지 확인
 */
export const isUrlencoded = (contentType: string): boolean => {
  if (!contentType) return false;
  return contentType.toLowerCase().startsWith("application/x-www-form-urlencoded");
};

// ─── Multipart 파싱 ──────────────────────────────────────────────────

/**
 * Content-Disposition 헤더에서 name과 filename을 추출
 */
const parseContentDisposition = (header: string): { name: string; fileName?: string } => {
  const nameMatch = header.match(/name="([^"]*)"/) || header.match(/name=([^\s;]+)/);
  const fileNameMatch = header.match(/filename="([^"]*)"/) || header.match(/filename=([^\s;]+)/);

  return {
    name: nameMatch ? nameMatch[1] : "",
    fileName: fileNameMatch ? fileNameMatch[1] : undefined,
  };
};

/**
 * 개별 파트의 헤더를 파싱
 */
const parsePartHeaders = (
  headerSection: string,
): { name: string; fileName?: string; contentType?: string } => {
  const lines = headerSection.split(/\r?\n/).filter(Boolean);
  let name = "";
  let fileName: string | undefined;
  let contentType: string | undefined;

  for (const line of lines) {
    const lowerLine = line.toLowerCase();
    if (lowerLine.startsWith("content-disposition:")) {
      const parsed = parseContentDisposition(line.substring("content-disposition:".length));
      name = parsed.name;
      fileName = parsed.fileName;
    } else if (lowerLine.startsWith("content-type:")) {
      contentType = line.substring("content-type:".length).trim();
    }
  }

  return { name, fileName, contentType };
};

/**
 * multipart/form-data 바디를 파싱하여 필드 목록을 반환한다.
 *
 * @param body - 원본 바디 데이터 (Uint8Array 또는 string)
 * @param boundary - Content-Type에서 추출한 boundary
 * @returns 파싱된 필드 배열
 */
/** Uint8Array에서 부분 바이트 시퀀스의 위치를 찾는다(없으면 -1). */
const indexOfBytes = (haystack: Uint8Array, needle: Uint8Array, from: number): number => {
  if (needle.length === 0 || haystack.length < needle.length) return -1;
  for (let i = from; i <= haystack.length - needle.length; i += 1) {
    let matched = true;
    for (let j = 0; j < needle.length; j += 1) {
      if (haystack[i + j] !== needle[j]) {
        matched = false;
        break;
      }
    }
    if (matched) return i;
  }
  return -1;
};

const CRLF_CRLF = new Uint8Array([0x0d, 0x0a, 0x0d, 0x0a]);
const LF_LF = new Uint8Array([0x0a, 0x0a]);

/**
 * multipart/form-data 바디를 "바이트 단위로" 파싱하여 필드 목록을 반환한다.
 *
 * 바디 전체를 UTF-8 문자열로 디코딩하면(특히 바이너리 파일 파트) 바이트 경계가 손실되어
 * 파일 크기가 부정확해진다. 따라서 boundary/헤더(ASCII)는 바이트로 탐색하고, 본문은
 * 원본 바이트 길이로 size를 계산한다. 텍스트 파트만 UTF-8로 디코딩해 value로 노출한다.
 */
export const parseMultipartFormData = (
  body: Uint8Array | string,
  boundary: string,
): MultipartField[] => {
  const encoder = new TextEncoder();
  const decoder = new TextDecoder("utf-8", { fatal: false });
  const bytes = typeof body === "string" ? encoder.encode(body) : body;
  const delimiter = encoder.encode(`--${boundary}`);

  // delimiter 위치들로 파트 경계(이전 delimiter 끝 ~ 다음 delimiter 시작)를 수집한다.
  const ranges: { start: number; end: number }[] = [];
  let prevEnd = -1;
  let searchFrom = 0;
  for (;;) {
    const idx = indexOfBytes(bytes, delimiter, searchFrom);
    if (idx === -1) break;
    if (prevEnd !== -1) ranges.push({ start: prevEnd, end: idx });
    prevEnd = idx + delimiter.length;
    searchFrom = prevEnd;
  }

  const fields: MultipartField[] = [];

  for (const range of ranges) {
    let start = range.start;
    let end = range.end;

    // delimiter 직후의 CRLF(또는 LF)와 다음 delimiter 직전의 CRLF(또는 LF) 제거
    if (bytes[start] === 0x0d && bytes[start + 1] === 0x0a) start += 2;
    else if (bytes[start] === 0x0a) start += 1;
    if (bytes[end - 2] === 0x0d && bytes[end - 1] === 0x0a) end -= 2;
    else if (bytes[end - 1] === 0x0a) end -= 1;

    if (end <= start) continue; // 빈 파트 또는 종료 마커("--")

    const part = bytes.subarray(start, end);

    // 헤더/본문 분리 (\r\n\r\n 우선, 없으면 \n\n)
    let headerEnd = indexOfBytes(part, CRLF_CRLF, 0);
    let bodyStart = headerEnd + 4;
    if (headerEnd === -1) {
      headerEnd = indexOfBytes(part, LF_LF, 0);
      bodyStart = headerEnd + 2;
    }
    if (headerEnd === -1) continue;

    const headerSection = decoder.decode(part.subarray(0, headerEnd));
    const bodyBytes = part.subarray(bodyStart);

    const { name, fileName, contentType } = parsePartHeaders(headerSection);
    const isFile = fileName !== undefined;

    fields.push({
      name,
      // 바이너리 파일 파트는 디코딩하지 않고, 텍스트 파트만 UTF-8로 디코딩
      value: isFile ? undefined : decoder.decode(bodyBytes),
      fileName,
      contentType,
      isFile,
      size: bodyBytes.length,
    });
  }

  return fields;
};

// ─── URL Encoded 파싱 ────────────────────────────────────────────────

/**
 * application/x-www-form-urlencoded 바디를 파싱하여 키-값 쌍을 반환한다.
 * URL 쿼리 파라미터에도 사용 가능.
 *
 * @param body - URL 인코딩된 문자열 (또는 Uint8Array)
 * @returns 파싱된 키-값 배열
 */
export const parseUrlencoded = (body: Uint8Array | string): UrlencodedField[] => {
  const text =
    typeof body === "string" ? body : new TextDecoder("utf-8", { fatal: false }).decode(body);

  const trimmed = text.trim();
  if (!trimmed) return [];

  // 쿼리 문자열 앞의 '?' 제거
  const queryString = trimmed.startsWith("?") ? trimmed.substring(1) : trimmed;

  if (!queryString) return [];

  return queryString
    .split("&")
    .filter(Boolean)
    .map((pair) => {
      const eqIndex = pair.indexOf("=");
      if (eqIndex === -1) {
        return {
          key: safeDecodeURIComponent(pair),
          value: "",
        };
      }
      return {
        key: safeDecodeURIComponent(pair.substring(0, eqIndex)),
        value: safeDecodeURIComponent(pair.substring(eqIndex + 1)),
      };
    });
};

/**
 * 안전한 decodeURIComponent - 잘못된 인코딩도 처리
 */
const safeDecodeURIComponent = (str: string): string => {
  try {
    return decodeURIComponent(str.replace(/\+/g, " "));
  } catch {
    return str;
  }
};
