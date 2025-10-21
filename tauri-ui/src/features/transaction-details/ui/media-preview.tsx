import { isImageDataType, isVideoDataType, isAudioDataType } from '@/entities/proxy/model/data-type';
import type { DataType } from '@/entities/proxy/model/data-type';

interface MediaPreviewProps {
  data: Uint8Array;
  dataType: DataType;
  className?: string;
  mimeType?: string; // MIME 타입 정보 추가
}

export const MediaPreview = ({ data, dataType, className, mimeType }: MediaPreviewProps) => {
  // Uint8Array를 Blob으로 변환 (MIME 타입 지정)
  const blob = new Blob([data], { type: mimeType || 'application/octet-stream' });
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
