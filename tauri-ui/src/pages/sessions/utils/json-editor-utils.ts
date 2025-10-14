/**
 * JSON 에디터를 위한 유틸리티 함수들
 */

/**
 * 값을 JSON 문자열로 포맷팅합니다
 * @param value - 포맷팅할 값
 * @returns 포맷팅된 JSON 문자열
 */
export const formatValueToJsonString = (value: unknown): string => {
  if (!value) return '';

  // 이미 문자열인 경우 파싱 시도
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return value;
    }
  }

  // 객체인 경우 그대로 문자열화
  return JSON.stringify(value, null, 2);
};

/**
 * 에디터 값 변경을 처리합니다
 * @param value - 에디터에서 입력된 값
 * @param isEditing - 편집 모드 여부
 * @param handleChange - 값 변경 핸들러
 */
export const handleEditorChange = <T>(
  value: string | undefined,
  isEditing: boolean,
  handleChange: (value: T) => void,
): void => {
  if (!isEditing || value === undefined) return;

  // 빈 텍스트인 경우 undefined로 저장
  if (value.trim() === '') {
    handleChange(undefined as T);
    return;
  }

  try {
    const parsed = JSON.parse(value);
    // 빈 객체인 경우 undefined로 저장
    if (parsed && typeof parsed === 'object' && Object.keys(parsed).length === 0) {
      handleChange(undefined as T);
    } else {
      handleChange(parsed as T);
    }
  } catch {
    // Invalid JSON, save as string
    handleChange(value as T);
  }
};
