import { isImageDataType, isVideoDataType, isAudioDataType } from '@/entities/proxy/model/data-type';
import type { DataType } from '@/entities/proxy/model/data-type';

interface MediaPreviewProps {
  data: Uint8Array;
  dataType: DataType;
  className?: string;
  mimeType?: string; // MIME 타입 정보 추가
}

// 파일 헤더를 읽어서 MIME 타입을 감지하는 함수
function detectMimeTypeFromHeader(data: Uint8Array): string {
  if (data.length < 4) {
    return 'application/octet-stream';
  }

  const header = data.slice(0, 4);
  
  // 이미지 파일 시그니처
  if (header[0] === 0xFF && header[1] === 0xD8 && header[2] === 0xFF) {
    return 'image/jpeg';
  }
  if (header[0] === 0x89 && header[1] === 0x50 && header[2] === 0x4E && header[3] === 0x47) {
    return 'image/png';
  }
  if (header[0] === 0x47 && header[1] === 0x49 && header[2] === 0x46 && header[3] === 0x38) {
    return 'image/gif';
  }
  if (header[0] === 0x42 && header[1] === 0x4D) {
    return 'image/bmp';
  }
  if (header[0] === 0x49 && header[1] === 0x49 && header[2] === 0x2A && header[3] === 0x00) {
    return 'image/tiff';
  }
  if (header[0] === 0x4D && header[1] === 0x4D && header[2] === 0x00 && header[3] === 0x2A) {
    return 'image/tiff';
  }
  if (header[0] === 0x00 && header[1] === 0x00 && header[2] === 0x01 && header[3] === 0x00) {
    return 'image/x-icon';
  }
  
  // WebP 체크 (더 많은 바이트 필요)
  if (data.length >= 12 && header[0] === 0x52 && header[1] === 0x49 && header[2] === 0x46 && header[3] === 0x46) {
    const webpHeader = data.slice(8, 12);
    if (String.fromCharCode(...webpHeader) === 'WEBP') {
      return 'image/webp';
    }
  }
  
  // 비디오 파일 시그니처
  if (header[0] === 0x00 && header[1] === 0x00 && header[2] === 0x00 && header[3] === 0x18) {
    if (data.length >= 8) {
      const ftypHeader = data.slice(4, 8);
      if (String.fromCharCode(...ftypHeader) === 'ftyp') {
        return 'video/mp4';
      }
    }
  }
  if (header[0] === 0x1A && header[1] === 0x45 && header[2] === 0xDF && header[3] === 0xA3) {
    return 'video/x-matroska';
  }
  
  // 오디오 파일 시그니처
  if (header[0] === 0x49 && header[1] === 0x44 && header[2] === 0x33) {
    return 'audio/mpeg';
  }
  if (header[0] === 0xFF && (header[1] === 0xFB || header[1] === 0xF3 || header[1] === 0xF2)) {
    return 'audio/mpeg';
  }
  if (header[0] === 0x4F && header[1] === 0x67 && header[2] === 0x67 && header[3] === 0x53) {
    return 'audio/ogg';
  }
  
  // WAV 체크
  if (data.length >= 12 && header[0] === 0x52 && header[1] === 0x49 && header[2] === 0x46 && header[3] === 0x46) {
    const waveHeader = data.slice(8, 12);
    if (String.fromCharCode(...waveHeader) === 'WAVE') {
      return 'audio/wav';
    }
  }
  
  // 문서 파일 시그니처
  if (header[0] === 0x25 && header[1] === 0x50 && header[2] === 0x44 && header[3] === 0x46) {
    return 'application/pdf';
  }
  
  // 압축 파일 시그니처
  if (header[0] === 0x50 && header[1] === 0x4B && (header[2] === 0x03 || header[2] === 0x05 || header[2] === 0x07)) {
    return 'application/zip';
  }
  if (header[0] === 0x52 && header[1] === 0x61 && header[2] === 0x72 && header[3] === 0x21) {
    return 'application/x-rar-compressed';
  }
  if (header[0] === 0x37 && header[1] === 0x7A && header[2] === 0xBC && header[3] === 0xAF) {
    return 'application/x-7z-compressed';
  }
  
  return 'application/octet-stream';
}

export const MediaPreview = ({ data, dataType, className, mimeType }: MediaPreviewProps) => {
  // MIME 타입 결정: 전달받은 mimeType -> 파일 헤더 감지 순으로 시도
  const detectedMimeType = mimeType && mimeType !== 'application/octet-stream' 
    ? mimeType 
    : detectMimeTypeFromHeader(data);
  
  // Uint8Array를 Blob으로 변환 (감지된 MIME 타입 사용)
  const blob = new Blob([data], { type: detectedMimeType });
  const url = URL.createObjectURL(blob);

  if (isImageDataType(dataType)) {
    return (
      <div className={className}>
        <img
          src={url}
          alt="Image preview"
          className="max-w-full max-h-full object-contain"
          onLoad={() => URL.revokeObjectURL(url)}
          onError={() => URL.revokeObjectURL(url)}
        />
      </div>
    );
  }

  if (isVideoDataType(dataType)) {
    return (
      <div className={className}>
        <video
          src={url}
          controls
          className="max-w-full max-h-full"
          onLoadedData={() => URL.revokeObjectURL(url)}
          onError={() => URL.revokeObjectURL(url)}
        >
          Your browser does not support the video tag.
        </video>
      </div>
    );
  }

  if (isAudioDataType(dataType)) {
    return (
      <div className={className}>
        <audio
          src={url}
          controls
          className="w-full"
          onLoadedData={() => URL.revokeObjectURL(url)}
          onError={() => URL.revokeObjectURL(url)}
        >
          Your browser does not support the audio tag.
        </audio>
      </div>
    );
  }

  // 미디어 파일이 아닌 경우
  return (
    <div className={className}>
      <p className="text-muted-foreground">미디어 파일이 아닙니다.</p>
    </div>
  );
};
