import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/shared/lib/data-type";

/**
 * HTTP 요청을 PowerShell Invoke-WebRequest 명령어로 변환
 */
export const generatePowerShellCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return "Invoke-WebRequest -Uri 'http://localhost' -Method GET";
  }

  const { method, uri, headers = {}, body, data_type } = request;

  const lines: string[] = [];

  lines.push(`Invoke-WebRequest -Uri '${escapePowerShellString(uri)}' \``);
  lines.push(`  -Method ${method.toUpperCase()}`);

  // 헤더 추가
  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerPairs = headerEntries
      .map(([key, value]) => `    '${escapePowerShellString(key)}' = '${escapePowerShellString(value)}'`)
      .join("\n");
    // 이전 줄에 백틱 추가
    lines[lines.length - 1] += " `";
    lines.push(`  -Headers @{\n${headerPairs}\n  }`);
  }

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
    const bodyText = decoder.decode(uint8Array);
    if (bodyText.trim()) {
      lines[lines.length - 1] += " `";
      if (data_type === "Json") {
        lines.push(`  -ContentType 'application/json' \``);
        lines.push(`  -Body '${escapePowerShellString(bodyText)}'`);
      } else {
        lines.push(`  -Body '${escapePowerShellString(bodyText)}'`);
      }
    }
  }

  return lines.join("\n");
};

const escapePowerShellString = (str: string): string => {
  return str.replace(/'/g, "''");
};
