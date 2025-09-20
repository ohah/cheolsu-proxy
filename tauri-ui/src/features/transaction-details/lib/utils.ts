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
  return new Date(timestamp / 1_000_000).toISOString();
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
