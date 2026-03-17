import type { HttpTransaction } from "@/entities/proxy";
import { type DataType, isTextBasedDataType } from "@/shared/lib/data-type";
import { inferExpectSchema } from "@/shared/lib/schema-inference";

/**
 * HTTP 트랜잭션을 Vitest/Jest/Bun 테스트 코드로 변환
 */
export const generateVitestCode = (transaction: HttpTransaction): string => {
  const { request, response } = transaction;

  if (!request) {
    return DEFAULT_CODE;
  }

  const { method, uri, headers = {}, body, data_type } = request;
  const methodUpper = method.toUpperCase();
  const urlPath = extractPath(uri);

  const lines: string[] = [];
  lines.push(`import { describe, it, expect } from "vitest";`);
  lines.push("");
  lines.push(`describe("${methodUpper} ${escapeString(urlPath)}", () => {`);
  lines.push(`  it("should return ${response?.status ?? 200}", async () => {`);

  // fetch 호출 생성
  const fetchLines = buildFetchCall(methodUpper, uri, headers, body, data_type);
  for (const line of fetchLines) {
    lines.push(`    ${line}`);
  }

  lines.push("");

  // status assertion
  const status = response?.status ?? 200;
  lines.push(`    expect(response.status).toBe(${status});`);

  // response body assertion
  if (response?.body_json !== undefined && response.body_json !== null) {
    lines.push("");
    lines.push(`    const data = await response.json();`);
    const schema = inferExpectSchema(response.body_json);
    lines.push(`    expect(data).toEqual(${schema});`);
  } else {
    lines.push("");
    lines.push(`    const data = await response.text();`);
    lines.push(`    expect(data).toBeDefined();`);
  }

  lines.push(`  });`);
  lines.push(`});`);
  lines.push("");

  return lines.join("\n");
};

const buildFetchCall = (
  method: string,
  uri: string,
  headers: Record<string, string>,
  body: Uint8Array | null | undefined,
  data_type: DataType | undefined,
): string[] => {
  const lines: string[] = [];
  const options: string[] = [];

  if (method !== "GET") {
    options.push(`      method: "${method}"`);
  }

  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerLines = headerEntries
      .map(([key, value]) => `        "${escapeString(key)}": "${escapeString(value)}"`)
      .join(",\n");
    options.push(`      headers: {\n${headerLines}\n      }`);
  }

  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const bodyText = decodeBody(body);
    if (bodyText.trim()) {
      if (data_type === "Json") {
        try {
          JSON.parse(bodyText);
          options.push(`      body: JSON.stringify(${bodyText})`);
        } catch {
          options.push(`      body: ${JSON.stringify(bodyText)}`);
        }
      } else {
        options.push(`      body: ${JSON.stringify(bodyText)}`);
      }
    }
  }

  if (options.length === 0) {
    lines.push(`const response = await fetch("${escapeString(uri)}");`);
  } else {
    lines.push(`const response = await fetch("${escapeString(uri)}", {`);
    lines.push(options.join(",\n"));
    lines.push(`});`);
  }

  return lines;
};

const extractPath = (uri: string): string => {
  try {
    const url = new URL(uri);
    return url.pathname + url.search;
  } catch {
    return uri;
  }
};

const escapeString = (str: string): string => {
  return str.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
};

const decodeBody = (body: Uint8Array | number[]): string => {
  const decoder = new TextDecoder("utf-8", { fatal: false });
  const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
  return decoder.decode(uint8Array);
};

const DEFAULT_CODE = `import { describe, it, expect } from "vitest";

describe("GET /", () => {
  it("should return 200", async () => {
    const response = await fetch("http://localhost");

    expect(response.status).toBe(200);

    const data = await response.text();
    expect(data).toBeDefined();
  });
});
`;
