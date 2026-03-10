import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";

/**
 * HTTP 요청을 HTTPie CLI 명령어로 변환
 */
export const generateHttpieCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return "http GET http://localhost";
  }

  const { method, uri, headers = {}, body, data_type } = request;

  let command = `http ${method.toUpperCase()}`;

  // URL 추가
  command += ` ${shellEscape(uri)}`;

  // 헤더 추가
  Object.entries(headers).forEach(([key, value]) => {
    command += ` \\\n  ${shellEscape(`${key}:${value}`)}`;
  });

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
    const bodyText = decoder.decode(uint8Array);
    if (bodyText.trim()) {
      if (data_type === "Json") {
        try {
          const parsed = JSON.parse(bodyText);
          // JSON 객체의 키-값 쌍을 HTTPie 형식으로 변환
          if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
            Object.entries(parsed).forEach(([key, value]) => {
              if (typeof value === "string") {
                command += ` \\\n  ${shellEscape(key)}=${shellEscape(value)}`;
              } else {
                // 비문자열 값은 := 사용 (HTTPie raw JSON 값)
                command += ` \\\n  ${shellEscape(key)}:=${shellEscape(JSON.stringify(value))}`;
              }
            });
          } else {
            // 배열이나 원시값인 경우 raw body로 전달
            command = `echo ${shellEscape(bodyText)} | ${command}`;
          }
        } catch {
          command = `echo ${shellEscape(bodyText)} | ${command}`;
        }
      } else {
        command = `echo ${shellEscape(bodyText)} | ${command}`;
      }
    }
  }

  return command;
};

const shellEscape = (str: string): string => {
  if (/^[a-zA-Z0-9_./:@=,{}[\]"#+-]+$/.test(str)) {
    return str;
  }
  return `'${str.replace(/'/g, "'\\''")}'`;
};
