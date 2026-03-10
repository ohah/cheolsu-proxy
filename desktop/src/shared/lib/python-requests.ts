import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";

/**
 * HTTP 요청을 Python requests 라이브러리 코드로 변환
 */
export const generatePythonRequestsCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return 'import requests\n\nresponse = requests.get("http://localhost")';
  }

  const { method, uri, headers = {}, body, data_type } = request;

  const lines: string[] = ["import requests"];
  lines.push("");

  const methodLower = method.toLowerCase();
  const args: string[] = [`"${escapePythonString(uri)}"`];

  // 헤더 추가
  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerLines = headerEntries
      .map(([key, value]) => `    "${escapePythonString(key)}": "${escapePythonString(value)}"`)
      .join(",\n");
    args.push(`headers={\n${headerLines}\n}`);
  }

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
    const bodyText = decoder.decode(uint8Array);
    if (bodyText.trim()) {
      if (data_type === "Json") {
        try {
          const parsed = JSON.parse(bodyText);
          args.push(`json=${toPythonDict(parsed)}`);
        } catch {
          args.push(`data="${escapePythonString(bodyText)}"`);
        }
      } else {
        args.push(`data="${escapePythonString(bodyText)}"`);
      }
    }
  }

  const argsStr = args.join(",\n  ");
  lines.push(`response = requests.${methodLower}(\n  ${argsStr}\n)`);

  return lines.join("\n");
};

const escapePythonString = (str: string): string => {
  return str
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\n/g, "\\n")
    .replace(/\r/g, "\\r")
    .replace(/\t/g, "\\t");
};

const toPythonDict = (value: unknown): string => {
  if (value === null) {
    return "None";
  }
  if (value === true) {
    return "True";
  }
  if (value === false) {
    return "False";
  }
  if (typeof value === "number") {
    return String(value);
  }
  if (typeof value === "string") {
    return `"${escapePythonString(value)}"`;
  }
  if (Array.isArray(value)) {
    const items = value.map((item) => toPythonDict(item)).join(", ");
    return `[${items}]`;
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .map(([key, val]) => `"${escapePythonString(key)}": ${toPythonDict(val)}`)
      .join(", ");
    return `{${entries}}`;
  }
  return String(value);
};
