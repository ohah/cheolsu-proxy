// 데이터 타입 열거형 (MITM 프록시에 최적화)
export type DataType =
  | "Json"
  | "Xml"
  | "Html"
  | "Text"
  | "Css"
  | "Javascript"
  | "Image"
  | "Video"
  | "Audio"
  | "Document"
  | "Archive"
  | "Binary"
  | "Empty"
  | "Unknown";

/**
 * 데이터 타입을 Monaco Editor 언어 모드로 변환
 */
export const dataTypeToMonacoLanguage = (dataType: DataType): string => {
  switch (dataType) {
    case "Json":
      return "json";
    case "Xml":
      return "xml";
    case "Html":
      return "html";
    case "Css":
      return "css";
    case "Javascript":
      return "javascript";
    case "Text":
      return "plaintext";
    case "Image":
    case "Video":
    case "Audio":
    case "Document":
    case "Archive":
    case "Binary":
    case "Empty":
    case "Unknown":
    default:
      return "plaintext";
  }
};

/**
 * 데이터 타입을 MIME 타입으로 변환
 */
export const dataTypeToMimeType = (dataType: DataType): string => {
  switch (dataType) {
    case "Json":
      return "application/json";
    case "Xml":
      return "application/xml";
    case "Html":
      return "text/html";
    case "Css":
      return "text/css";
    case "Javascript":
      return "application/javascript";
    case "Text":
      return "text/plain";
    case "Image":
      return "image/*";
    case "Video":
      return "video/*";
    case "Audio":
      return "audio/*";
    case "Document":
      return "application/pdf";
    case "Archive":
      return "application/zip";
    case "Binary":
      return "application/octet-stream";
    case "Empty":
      return "empty";
    case "Unknown":
    default:
      return "application/octet-stream";
  }
};

/**
 * 데이터 타입을 사용자 친화적인 이름으로 변환
 */
export const dataTypeToDisplayName = (dataType: DataType): string => {
  switch (dataType) {
    case "Json":
      return "JSON";
    case "Xml":
      return "XML";
    case "Html":
      return "HTML";
    case "Css":
      return "CSS";
    case "Javascript":
      return "JavaScript";
    case "Text":
      return "Plain Text";
    case "Image":
      return "Image";
    case "Video":
      return "Video";
    case "Audio":
      return "Audio";
    case "Document":
      return "Document";
    case "Archive":
      return "Archive";
    case "Binary":
      return "Binary Data";
    case "Empty":
      return "Empty";
    case "Unknown":
    default:
      return "Unknown";
  }
};

/**
 * 데이터 타입에 따른 아이콘 반환
 */
export const dataTypeToIcon = (dataType: DataType): string => {
  switch (dataType) {
    case "Json":
      return "📄";
    case "Xml":
      return "📄";
    case "Html":
      return "🌐";
    case "Css":
      return "🎨";
    case "Javascript":
      return "⚡";
    case "Text":
      return "📄";
    case "Image":
      return "🖼️";
    case "Video":
      return "🎬";
    case "Audio":
      return "🎵";
    case "Document":
      return "📕";
    case "Archive":
      return "📦";
    case "Binary":
      return "📦";
    case "Empty":
      return "📭";
    case "Unknown":
    default:
      return "❓";
  }
};

/**
 * 데이터 타입이 텍스트 기반인지 확인
 */
export const isTextBasedDataType = (dataType: DataType): boolean => {
  return ["Json", "Xml", "Html", "Css", "Javascript", "Text"].includes(dataType);
};

/**
 * 데이터 타입이 이미지인지 확인
 */
export const isImageDataType = (dataType: DataType): boolean => {
  return dataType === "Image";
};

/**
 * 데이터 타입이 비디오인지 확인
 */
export const isVideoDataType = (dataType: DataType): boolean => {
  return dataType === "Video";
};

/**
 * 데이터 타입이 오디오인지 확인
 */
export const isAudioDataType = (dataType: DataType): boolean => {
  return dataType === "Audio";
};

/**
 * 데이터 타입이 문서인지 확인
 */
export const isDocumentDataType = (dataType: DataType): boolean => {
  return dataType === "Document";
};

/**
 * 데이터 타입이 압축 파일인지 확인
 */
export const isArchiveDataType = (dataType: DataType): boolean => {
  return dataType === "Archive";
};

/**
 * 데이터 타입이 압축된 데이터인지 확인 (Archive와 동일)
 */
export const isCompressedDataType = (dataType: DataType): boolean => {
  return dataType === "Archive";
};

/**
 * 데이터 타입이 바이너리인지 확인
 */
export const isBinaryDataType = (dataType: DataType): boolean => {
  return ["Image", "Video", "Audio", "Document", "Archive", "Binary"].includes(dataType);
};

/**
 * 데이터 타입이 미디어 파일인지 확인 (이미지, 비디오, 오디오)
 * 이 타입들은 무조건 파일로 저장됨
 */
export const isMediaDataType = (dataType: DataType): boolean => {
  return ["Image", "Video", "Audio"].includes(dataType);
};
