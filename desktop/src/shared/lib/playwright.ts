import type { HttpTransaction } from "@/entities/proxy";
import { type DataType, isTextBasedDataType } from "@/shared/lib/data-type";
import { inferExpectSchema } from "@/shared/lib/schema-inference";

/**
 * HTTP 트랜잭션을 Playwright API 테스트 코드로 변환
 */
export const generatePlaywrightCode = (transaction: HttpTransaction): string => {
  const { request, response } = transaction;

  if (!request) {
    return DEFAULT_CODE;
  }

  const { method, uri, headers = {}, body, data_type } = request;
  const methodUpper = method.toUpperCase();
  const methodLower = method.toLowerCase();
  const urlPath = extractPath(uri);
  const status = response?.status ?? 200;

  const lines: string[] = [];
  lines.push(`import { test, expect } from "@playwright/test";`);
  lines.push("");
  lines.push(
    `test("${methodUpper} ${escapeString(urlPath)} should return ${status}", async ({ request }) => {`,
  );

  // request call 생성
  const options = buildPlaywrightOptions(headers, body, data_type);

  if (options.length === 0) {
    lines.push(`  const response = await request.${methodLower}("${escapeString(uri)}");`);
  } else {
    lines.push(`  const response = await request.${methodLower}("${escapeString(uri)}", {`);
    for (const opt of options) {
      lines.push(`    ${opt}`);
    }
    lines.push(`  });`);
  }

  lines.push("");
  lines.push(`  expect(response.status()).toBe(${status});`);

  // response body assertion
  if (response?.body_json !== undefined && response.body_json !== null) {
    lines.push("");
    lines.push(`  const data = await response.json();`);
    const schema = inferExpectSchema(response.body_json, 2);
    lines.push(`  expect(data).toEqual(${schema});`);
  }

  lines.push(`});`);
  lines.push("");

  return lines.join("\n");
};

const buildPlaywrightOptions = (
  headers: Record<string, string>,
  body: Uint8Array | null | undefined,
  data_type: DataType | undefined,
): string[] => {
  const options: string[] = [];

  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerLines = headerEntries
      .map(([key, value]) => `      "${escapeString(key)}": "${escapeString(value)}"`)
      .join(",\n");
    options.push(`headers: {\n${headerLines}\n    },`);
  }

  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const bodyText = decodeBody(body);
    if (bodyText.trim()) {
      if (data_type === "Json") {
        try {
          const parsed = JSON.parse(bodyText);
          options.push(`data: ${JSON.stringify(parsed)},`);
        } catch {
          options.push(`data: ${JSON.stringify(bodyText)},`);
        }
      } else {
        options.push(`data: ${JSON.stringify(bodyText)},`);
      }
    }
  }

  return options;
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

const DEFAULT_CODE = `import { test, expect } from "@playwright/test";

test("GET / should return 200", async ({ request }) => {
  const response = await request.get("http://localhost");

  expect(response.status()).toBe(200);
});
`;
