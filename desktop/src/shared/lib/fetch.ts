import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/shared/lib/data-type";

/**
 * HTTP 요청을 JavaScript fetch API 코드로 변환
 */
export const generateFetchCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return `fetch("http://localhost")`;
  }

  const { method, uri, headers = {}, body, data_type } = request;

  const options: string[] = [];

  // 메서드 추가 (GET이 아닌 경우)
  if (method.toUpperCase() !== "GET") {
    options.push(`  method: "${method.toUpperCase()}"`);
  }

  // 헤더 추가
  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerLines = headerEntries
      .map(([key, value]) => `    "${escapeString(key)}": "${escapeString(value)}"`)
      .join(",\n");
    options.push(`  headers: {\n${headerLines}\n  }`);
  }

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
    const bodyText = decoder.decode(uint8Array);
    if (bodyText.trim()) {
      if (data_type === "Json") {
        try {
          // JSON인 경우 JSON.stringify로 감싸기
          JSON.parse(bodyText);
          options.push(`  body: JSON.stringify(${bodyText})`);
        } catch {
          options.push(`  body: ${JSON.stringify(bodyText)}`);
        }
      } else {
        options.push(`  body: ${JSON.stringify(bodyText)}`);
      }
    }
  }

  if (options.length === 0) {
    return `fetch("${escapeString(uri)}")`;
  }

  return `fetch("${escapeString(uri)}", {\n${options.join(",\n")}\n})`;
};

const escapeString = (str: string): string => {
  return str.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
};
