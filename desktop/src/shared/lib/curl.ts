import type { HttpTransaction } from "@/entities/proxy";
import { isTextBasedDataType } from "@/entities/proxy/model/data-type";

/**
 * HTTP 요청을 cURL 명령어로 변환
 */
export const generateCurlCommand = (transaction: HttpTransaction): string => {
  const { request } = transaction;

  if (!request) {
    return 'curl -X GET "http://localhost"';
  }

  const { method, uri, headers = {}, body, data_type } = request;

  let curlCommand = `curl -X ${method.toUpperCase()}`;

  // 헤더 추가
  Object.entries(headers).forEach(([key, value]) => {
    curlCommand += ` \\\n  -H "${key}: ${value}"`;
  });

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const decoder = new TextDecoder("utf-8", { fatal: false });
    const uint8Array = body instanceof Uint8Array ? body : new Uint8Array(body);
    const bodyText = decoder.decode(uint8Array);
    if (bodyText.trim()) {
      curlCommand += ` \\\n  -d '${bodyText.replace(/'/g, "\\'")}'`;
    }
  }

  // URL 추가
  curlCommand += ` \\\n  "${uri}"`;

  return curlCommand;
};
