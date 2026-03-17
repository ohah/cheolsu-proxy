import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/shared/lib/data-type";
import { inferK6Checks } from "@/shared/lib/schema-inference";

/**
 * HTTP 트랜잭션을 k6 로드테스트 코드로 변환
 */
export const generateK6Code = (transaction: HttpTransaction): string => {
  const { request, response } = transaction;

  if (!request) {
    return DEFAULT_CODE;
  }

  const { method, uri, headers = {}, body, data_type } = request;
  const methodUpper = method.toUpperCase();
  const status = response?.status ?? 200;

  const lines: string[] = [];
  lines.push(`import http from "k6/http";`);
  lines.push(`import { check, sleep } from "k6";`);
  lines.push("");
  lines.push(`export const options = {`);
  lines.push(`  vus: 10,`);
  lines.push(`  duration: "30s",`);
  lines.push(`};`);
  lines.push("");
  lines.push(`export default function () {`);

  // 헤더
  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    lines.push(`  const headers = {`);
    for (const [key, value] of headerEntries) {
      lines.push(`    "${escapeString(key)}": "${escapeString(value)}",`);
    }
    lines.push(`  };`);
    lines.push("");
  }

  // body
  let hasBody = false;
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const bodyText = decodeBody(body);
    if (bodyText.trim()) {
      hasBody = true;
      if (data_type === "Json") {
        try {
          JSON.parse(bodyText);
          lines.push(`  const payload = JSON.stringify(${bodyText});`);
        } catch {
          lines.push(`  const payload = ${JSON.stringify(bodyText)};`);
        }
      } else {
        lines.push(`  const payload = ${JSON.stringify(bodyText)};`);
      }
      lines.push("");
    }
  }

  // HTTP 호출
  const hasHeaders = headerEntries.length > 0;
  const paramsArg = hasHeaders ? ", { headers }" : "";
  const k6Method = methodUpper.toLowerCase();

  if (hasBody) {
    lines.push(`  const res = http.${k6Method}("${escapeString(uri)}", payload${paramsArg});`);
  } else if (k6Method === "get" || k6Method === "head" || k6Method === "options") {
    lines.push(
      `  const res = http.${k6Method}("${escapeString(uri)}"${hasHeaders ? ", { headers }" : ""});`,
    );
  } else {
    lines.push(`  const res = http.${k6Method}("${escapeString(uri)}", null${paramsArg});`);
  }

  lines.push("");

  // check assertions
  const checks: string[] = [];
  checks.push(`  "status is ${status}": (r) => r.status === ${status}`);

  if (response?.body_json !== undefined && response.body_json !== null) {
    const jsonChecks = inferK6Checks(response.body_json);
    checks.push(...jsonChecks);
  }

  lines.push(`  check(res, {`);
  lines.push(checks.join(",\n") + ",");
  lines.push(`  });`);
  lines.push("");
  lines.push(`  sleep(1);`);
  lines.push(`}`);
  lines.push("");

  return lines.join("\n");
};

const escapeString = (str: string): string => {
  return str.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
};

const decodeBody = (body: Uint8Array | number[]): string => {
  const decoder = new TextDecoder("utf-8", { fatal: false });
  const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
  return decoder.decode(uint8Array);
};

const DEFAULT_CODE = `import http from "k6/http";
import { check, sleep } from "k6";

export const options = {
  vus: 10,
  duration: "30s",
};

export default function () {
  const res = http.get("http://localhost");

  check(res, {
    "status is 200": (r) => r.status === 200,
  });

  sleep(1);
}
`;
