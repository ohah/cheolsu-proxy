import { DataType } from '@/entities/proxy/model/types';
import { isTextBasedDataType, isBinaryDataType } from '@/entities/proxy/model/data-type';

/**
 * Uint8Array를 문자열로 변환 (UTF-8 디코딩)
 * 러스트에서 이미 GZIP 압축 해제와 데이터 타입 감지를 완료했으므로 단순한 UTF-8 디코딩만 수행
 */
export const uint8ArrayToString = (data: Uint8Array | number[], dataType: DataType): string => {
  if (!data || data.length === 0) {
    return '';
  }

  try {
    // 일반 배열인 경우 Uint8Array로 변환
    const uint8Array = data instanceof Uint8Array ? data : new Uint8Array(data);

    // 러스트에서 이미 처리된 데이터이므로 단순한 UTF-8 디코딩
    const decoder = new TextDecoder('utf-8', { fatal: false });
    return decoder.decode(uint8Array);
  } catch (error) {
    console.error('UTF-8 디코딩 실패:', error);
    return `디코딩 실패 (${dataType})`;
  }
};

/**
 * HTML 엔티티 디코딩
 */
export const decodeHtmlEntities = (text: string): string => {
  const textarea = document.createElement('textarea');
  textarea.innerHTML = text;
  return textarea.value;
};

/**
 * 요청/응답 본문을 포맷팅된 문자열로 변환
 * 러스트에서 이미 데이터 타입 감지와 압축 해제를 완료했으므로 단순한 포맷팅만 수행
 */
export const formatBodyContent = (body: Uint8Array, dataType: DataType, bodyJson?: any): string => {
  if (dataType === 'Empty') {
    return '';
  }

  // JSON 타입이고 body_json이 있으면 바로 포맷팅
  if (dataType === 'Json' && bodyJson) {
    return JSON.stringify(bodyJson, null, 2);
  }

  if (isTextBasedDataType(dataType)) {
    const text = uint8ArrayToString(body, dataType);

    // JSON 타입인 경우 포맷팅 시도 (fallback)
    if (dataType === 'Json') {
      try {
        const parsed = JSON.parse(text);
        return JSON.stringify(parsed, null, 2);
      } catch (error) {
        console.warn('JSON 파싱 실패, 원본 텍스트 반환:', error);
        return decodeHtmlEntities(text);
      }
    }

    return decodeHtmlEntities(text);
  }

  if (isBinaryDataType(dataType)) {
    return `[${dataType} - ${body.length} bytes]`;
  }

  return uint8ArrayToString(body, dataType);
};

/**
 * 요청/응답 본문을 표시용으로 변환 (Monaco Editor용)
 */
export const getBodyForDisplay = (body: Uint8Array, dataType: DataType, bodyJson?: any): string => {
  if (dataType === 'Empty') {
    return '';
  }

  if (isTextBasedDataType(dataType)) {
    return formatBodyContent(body, dataType, bodyJson);
  }

  if (isBinaryDataType(dataType)) {
    return `// ${dataType} 파일 (${body.length} bytes)\n// 이 파일은 바이너리 형식이므로 텍스트로 표시할 수 없습니다.`;
  }

  return formatBodyContent(body, dataType, bodyJson);
};

/**
 * HTTP 요청을 cURL 명령어로 변환
 */
export const generateCurlCommand = (request: {
  method: string;
  uri: string;
  headers?: Record<string, string>;
  body?: Uint8Array;
  data_type?: DataType;
}): string => {
  const { method, uri, headers = {}, body, data_type } = request;

  let curlCommand = `curl -X ${method.toUpperCase()}`;

  // 헤더 추가
  Object.entries(headers).forEach(([key, value]) => {
    curlCommand += ` \\\n  -H "${key}: ${value}"`;
  });

  // 바디 추가 (텍스트 기반 데이터인 경우)
  if (body && body.length > 0 && data_type && isTextBasedDataType(data_type)) {
    const bodyText = uint8ArrayToString(body, data_type);
    if (bodyText.trim()) {
      curlCommand += ` \\\n  -d '${bodyText.replace(/'/g, "\\'")}'`;
    }
  }

  // URL 추가
  curlCommand += ` \\\n  "${uri}"`;

  return curlCommand;
};

// Re-export data type utilities for convenience
export {
  dataTypeToMonacoLanguage,
  dataTypeToMimeType,
  dataTypeToDisplayName,
  dataTypeToIcon,
  isTextBasedDataType,
  isImageDataType,
  isVideoDataType,
  isAudioDataType,
  isCompressedDataType,
  isBinaryDataType,
} from '@/entities/proxy/model/data-type';
