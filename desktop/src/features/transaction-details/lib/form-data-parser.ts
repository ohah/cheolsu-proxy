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
  return contentType.toLowerCase().startsWith("multipart/form-data");
};

/**
 * Content-Type이 application/x-www-form-urlencoded인지 확인
 */
export const isUrlencoded = (contentType: string): boolean => {
  return contentType.toLowerCase().startsWith("application/x-www-form-urlencoded");
};

// ─── Multipart 파싱 ──────────────────────────────────────────────────

/**
 * Content-Disposition 헤더에서 name과 filename을 추출
 */
const parseContentDisposition = (
  header: string,
): { name: string; fileName?: string } => {
  const nameMatch = header.match(/name="([^"]*)"/) || header.match(/name=([^\s;]+)/);
  const fileNameMatch =
    header.match(/filename="([^"]*)"/) || header.match(/filename=([^\s;]+)/);

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
  const lines = headerSection.split("\r\n").filter(Boolean);
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
export const parseMultipartFormData = (
  body: Uint8Array | string,
  boundary: string,
): MultipartField[] => {
  const text = typeof body === "string" ? body : new TextDecoder("utf-8", { fatal: false }).decode(body);

  const delimiter = `--${boundary}`;
  const closeDelimiter = `--${boundary}--`;

  // 파트 분리
  const parts = text.split(delimiter).filter((part) => {
    const trimmed = part.trim();
    return trimmed !== "" && trimmed !== "--" && !trimmed.startsWith("--\r\n") && trimmed !== "--";
  });

  const fields: MultipartField[] = [];

  for (const part of parts) {
    // 종료 구분자 이후는 무시
    if (part.trim() === "--" || part.trim().startsWith("--")) {
      continue;
    }

    // 헤더와 바디를 분리 (\r\n\r\n 또는 \n\n)
    let headerEnd = part.indexOf("\r\n\r\n");
    let bodyStart = headerEnd + 4;

    if (headerEnd === -1) {
      headerEnd = part.indexOf("\n\n");
      bodyStart = headerEnd + 2;
    }

    if (headerEnd === -1) continue;

    const headerSection = part.substring(0, headerEnd);
    let bodyContent = part.substring(bodyStart);

    // 마지막 줄바꿈 제거 (경계 구분자 전의 \r\n)
    if (bodyContent.endsWith("\r\n")) {
      bodyContent = bodyContent.slice(0, -2);
    } else if (bodyContent.endsWith("\n")) {
      bodyContent = bodyContent.slice(0, -1);
    }

    // 종료 구분자 제거
    const closeIdx = bodyContent.indexOf(closeDelimiter);
    if (closeIdx !== -1) {
      bodyContent = bodyContent.substring(0, closeIdx);
      if (bodyContent.endsWith("\r\n")) {
        bodyContent = bodyContent.slice(0, -2);
      }
    }

    const { name, fileName, contentType } = parsePartHeaders(headerSection);

    const isFile = fileName !== undefined;
    const size = new TextEncoder().encode(bodyContent).length;

    fields.push({
      name,
      value: isFile ? undefined : bodyContent,
      fileName,
      contentType,
      isFile,
      size,
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
  const text = typeof body === "string" ? body : new TextDecoder("utf-8", { fatal: false }).decode(body);

  const trimmed = text.trim();
  if (!trimmed) return [];

  // 쿼리 문자열 앞의 '?' 제거
  const queryString = trimmed.startsWith("?") ? trimmed.substring(1) : trimmed;

  if (!queryString) return [];

  return queryString.split("&").filter(Boolean).map((pair) => {
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
