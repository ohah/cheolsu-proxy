import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/shared/lib/data-type";

/**
 * HTTP 요청을 Node.js fetch 코드로 변환 (async/await 스타일)
 */
export const generateNodejsFetchCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return `const response = await fetch("http://localhost");\nconst data = await response.text();\nconsole.log(data);`;
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

  const lines: string[] = [];

  if (options.length === 0) {
    lines.push(`const response = await fetch("${escapeString(uri)}");`);
  } else {
    lines.push(`const response = await fetch("${escapeString(uri)}", {`);
    lines.push(options.join(",\n"));
    lines.push(`});`);
  }

  lines.push(`const data = await response.json();`);
  lines.push(`console.log(data);`);

  return lines.join("\n");
};

const escapeString = (str: string): string => {
  return str.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
};
