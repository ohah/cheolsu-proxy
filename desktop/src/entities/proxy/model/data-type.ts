// shared 레이어에서 re-export (FSD 규칙 준수)
export {
  type DataType,
  dataTypeToMonacoLanguage,
  dataTypeToMimeType,
  dataTypeToDisplayName,
  dataTypeToIcon,
  isTextBasedDataType,
  isImageDataType,
  isVideoDataType,
  isAudioDataType,
  isDocumentDataType,
  isArchiveDataType,
  isCompressedDataType,
  isBinaryDataType,
  isProtobufDataType,
  isGrpcDataType,
  isMediaDataType,
} from "@/shared/lib/data-type";
