export const formatDistanceToNowKr = (date: Date | number): string => {
  const now = new Date();
  const target = typeof date === 'number' ? new Date(date) : date;
  const diff = now.getTime() - target.getTime();

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(diff / (1000 * 60));
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const months = Math.floor(days / 30);
  const years = Math.floor(days / 365);

  if (seconds < 5) return '방금 전';
  if (seconds < 60) return `${seconds}초 전`;
  if (minutes < 60) return `${minutes}분 전`;
  if (hours < 24) return `${hours}시간 전`;
  if (days < 30) return `${days}일 전`;
  if (months < 12) return `${months}개월 전`;
  return `${years}년 전`;
};

export const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp / 1_000_000);

  return date.toLocaleString('ko-KR', {
    timeZone: 'Asia/Seoul',
  });
};

// HTML 엔티티 디코딩 함수
const decodeHtmlEntities = (text: string): string => {
  const textarea = document.createElement('textarea');
  textarea.innerHTML = text;
  return textarea.value;
};

export const formatBody = (body: Uint8Array | number[]): string => {
  try {
    const uint8Array = Array.isArray(body) ? new Uint8Array(body) : body;
    const text = new TextDecoder('utf-8', { fatal: true }).decode(uint8Array);

    // HTML 엔티티 디코딩
    const decodedText = decodeHtmlEntities(text);

    // JSON 포맷팅 시도
    try {
      const json = JSON.parse(decodedText);
      return JSON.stringify(json, null, 2);
    } catch {
      return decodedText;
    }
  } catch {
    const bytes = Array.isArray(body) ? body : Array.from(body);
    return `Binary data (${bytes.length} bytes): ${bytes.join(', ')}`;
  }
};

export const formatBodyToJson = (body: Uint8Array | any): Record<string, unknown> | string => {
  try {
    const uint8Array = Array.isArray(body) ? new Uint8Array(body) : body;
    const text = new TextDecoder('utf-8', { fatal: true }).decode(uint8Array);

    // HTML 엔티티 디코딩
    const decodedText = decodeHtmlEntities(text);

    // JSON 포맷팅 시도
    try {
      const json = JSON.parse(decodedText);
      return json;
    } catch {
      return decodedText;
    }
  } catch {
    // TODO 바이너리 데이터는 우선 편집 불가능함. @ohah
    return '';
  }
};

// JSON 타입 감지 함수
export const detectContentType = (content: string): 'json' | 'text' => {
  if (!content || content.trim().length === 0) {
    return 'text';
  }

  try {
    JSON.parse(content);
    return 'json';
  } catch {
    return 'text';
  }
};

// curl 명령어 생성 함수
export const generateCurlCommand = (transaction: any): string => {
  const { request } = transaction;
  if (!request) return '';

  const method = request.method || 'GET';
  const url = request.uri || '';
  const headers = request.headers || {};
  const body = request.body;

  let curlCommand = `curl -X ${method}`;

  // 헤더를 중요도 순으로 정렬하여 추가
  const sortedHeaders = Object.entries(headers).sort(([keyA], [keyB]) => {
    const priority = ['accept', 'authorization', 'content-type', 'user-agent'];
    const indexA = priority.indexOf(keyA.toLowerCase());
    const indexB = priority.indexOf(keyB.toLowerCase());

    if (indexA === -1 && indexB === -1) return keyA.localeCompare(keyB);
    if (indexA === -1) return 1;
    if (indexB === -1) return -1;
    return indexA - indexB;
  });

  // 헤더 추가
  sortedHeaders.forEach(([key, value]) => {
    curlCommand += ` \\\n  -H "${key}: ${value}"`;
  });

  // Body가 있는 경우에만 Content-Type 추가 (기본값이 없는 경우)
  if (body && body.length > 0 && !headers['Content-Type'] && !headers['content-type']) {
    curlCommand += ` \\\n  -H "Content-Type: application/json"`;
  }

  // URL 추가
  curlCommand += ` \\\n  "${url}"`;

  // Body 추가
  if (body && body.length > 0) {
    const bodyText = formatBody(body);
    // JSON인 경우 한 줄로 압축
    const compressedBody = bodyText.replace(/\s+/g, ' ').trim();
    curlCommand += ` \\\n  -d '${compressedBody}'`;
  }

  return curlCommand;
};
