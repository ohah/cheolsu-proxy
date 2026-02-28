import {
  isImageDataType,
  isVideoDataType,
  isAudioDataType,
} from "@/entities/proxy/model/data-type";
import type { DataType } from "@/entities/proxy/model/data-type";
import { readFile, BaseDirectory } from "@tauri-apps/plugin-fs";
import { useState, useEffect } from "react";

interface MediaPreviewProps {
  data?: Uint8Array; // 파일 경로가 있으면 선택적
  dataType: DataType;
  className?: string;
  mimeType?: string; // MIME 타입 정보 추가
  filePath?: string; // 파일 경로 추가
}

// 파일 헤더를 읽어서 파일 확장자를 감지하는 함수
function detectFileExtensionFromHeader(data: Uint8Array): string | null {
  if (data.length < 4) {
    return null;
  }

  const header = data.slice(0, 4);

  // 이미지 파일 시그니처
  if (header[0] === 0xff && header[1] === 0xd8 && header[2] === 0xff) {
    return "jpg";
  }
  if (header[0] === 0x89 && header[1] === 0x50 && header[2] === 0x4e && header[3] === 0x47) {
    return "png";
  }
  if (header[0] === 0x47 && header[1] === 0x49 && header[2] === 0x46 && header[3] === 0x38) {
    return "gif";
  }
  if (header[0] === 0x42 && header[1] === 0x4d) {
    return "bmp";
  }
  if (header[0] === 0x49 && header[1] === 0x49 && header[2] === 0x2a && header[3] === 0x00) {
    return "tiff";
  }
  if (header[0] === 0x4d && header[1] === 0x4d && header[2] === 0x00 && header[3] === 0x2a) {
    return "tiff";
  }
  if (header[0] === 0x00 && header[1] === 0x00 && header[2] === 0x01 && header[3] === 0x00) {
    return "ico";
  }

  // WebP 체크 (더 많은 바이트 필요)
  if (
    data.length >= 12 &&
    header[0] === 0x52 &&
    header[1] === 0x49 &&
    header[2] === 0x46 &&
    header[3] === 0x46
  ) {
    const webpHeader = data.slice(8, 12);
    if (String.fromCharCode(...webpHeader) === "WEBP") {
      return "webp";
    }
  }

  // 비디오 파일 시그니처
  if (header[0] === 0x00 && header[1] === 0x00 && header[2] === 0x00 && header[3] === 0x18) {
    if (data.length >= 8) {
      const ftypHeader = data.slice(4, 8);
      if (String.fromCharCode(...ftypHeader) === "ftyp") {
        return "mp4";
      }
    }
  }
  if (header[0] === 0x1a && header[1] === 0x45 && header[2] === 0xdf && header[3] === 0xa3) {
    return "mkv";
  }

  // 오디오 파일 시그니처
  if (header[0] === 0x49 && header[1] === 0x44 && header[2] === 0x33) {
    return "mp3";
  }
  if (header[0] === 0xff && (header[1] === 0xfb || header[1] === 0xf3 || header[1] === 0xf2)) {
    return "mp3";
  }
  if (header[0] === 0x4f && header[1] === 0x67 && header[2] === 0x67 && header[3] === 0x53) {
    return "ogg";
  }

  // WAV 체크
  if (
    data.length >= 12 &&
    header[0] === 0x52 &&
    header[1] === 0x49 &&
    header[2] === 0x46 &&
    header[3] === 0x46
  ) {
    const waveHeader = data.slice(8, 12);
    if (String.fromCharCode(...waveHeader) === "WAVE") {
      return "wav";
    }
  }

  // 문서 파일 시그니처
  if (header[0] === 0x25 && header[1] === 0x50 && header[2] === 0x44 && header[3] === 0x46) {
    return "pdf";
  }

  // 압축 파일 시그니처
  if (
    header[0] === 0x50 &&
    header[1] === 0x4b &&
    (header[2] === 0x03 || header[2] === 0x05 || header[2] === 0x07)
  ) {
    return "zip";
  }
  if (header[0] === 0x52 && header[1] === 0x61 && header[2] === 0x72 && header[3] === 0x21) {
    return "rar";
  }
  if (header[0] === 0x37 && header[1] === 0x7a && header[2] === 0xbc && header[3] === 0xaf) {
    return "7z";
  }

  return null;
}

// 파일 확장자에서 MIME 타입을 추출하는 함수
function getMimeTypeFromExtension(extension: string): string {
  switch (extension) {
    // 이미지
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "png":
      return "image/png";
    case "gif":
      return "image/gif";
    case "webp":
      return "image/webp";
    case "svg":
      return "image/svg+xml";
    case "bmp":
      return "image/bmp";
    case "tiff":
      return "image/tiff";
    case "ico":
      return "image/x-icon";

    // 비디오
    case "mp4":
      return "video/mp4";
    case "avi":
      return "video/avi";
    case "mov":
      return "video/quicktime";
    case "wmv":
      return "video/x-ms-wmv";
    case "flv":
      return "video/x-flv";
    case "webm":
      return "video/webm";
    case "mkv":
      return "video/x-matroska";
    case "3gp":
      return "video/3gpp";

    // 오디오
    case "mp3":
      return "audio/mpeg";
    case "wav":
      return "audio/wav";
    case "ogg":
      return "audio/ogg";
    case "aac":
      return "audio/aac";
    case "flac":
      return "audio/flac";
    case "m4a":
      return "audio/mp4";
    case "wma":
      return "audio/x-ms-wma";

    // 문서
    case "pdf":
      return "application/pdf";
    case "doc":
      return "application/msword";
    case "docx":
      return "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
    case "xls":
      return "application/vnd.ms-excel";
    case "xlsx":
      return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
    case "ppt":
      return "application/vnd.ms-powerpoint";
    case "pptx":
      return "application/vnd.openxmlformats-officedocument.presentationml.presentation";

    // 압축
    case "zip":
      return "application/zip";
    case "rar":
      return "application/x-rar-compressed";
    case "7z":
      return "application/x-7z-compressed";
    case "gz":
      return "application/gzip";
    case "tar":
      return "application/x-tar";

    // 기타
    case "txt":
      return "text/plain";
    case "html":
      return "text/html";
    case "css":
      return "text/css";
    case "js":
      return "application/javascript";
    case "json":
      return "application/json";
    case "xml":
      return "application/xml";

    default:
      return "application/octet-stream";
  }
}

export const MediaPreview = ({
  data,
  dataType,
  className,
  mimeType,
  filePath,
}: MediaPreviewProps) => {
  const [mediaUrl, setMediaUrl] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadMedia = async () => {
      setLoading(true);
      setError(null);

      try {
        let fileData: Uint8Array;
        let detectedMimeType: string;

        if (filePath) {
          // Tauri File System API로 파일 직접 읽기 (상대 경로 + baseDir 방식)
          // Rust에서 이미 상대 경로로 전달되므로 그대로 사용
          const rawData = await readFile(filePath, { baseDir: BaseDirectory.Cache });

          fileData = new Uint8Array(rawData);

          // MIME 타입 결정
          detectedMimeType = mimeType && mimeType !== "application/octet-stream" ? mimeType : "";

          if (!detectedMimeType) {
            const detectedExtension = detectFileExtensionFromHeader(fileData);
            detectedMimeType = detectedExtension
              ? getMimeTypeFromExtension(detectedExtension)
              : "application/octet-stream";
          }
        } else if (data) {
          // 기존 방식 (메모리 데이터)
          fileData = data;
          detectedMimeType = mimeType && mimeType !== "application/octet-stream" ? mimeType : "";

          if (!detectedMimeType) {
            const detectedExtension = detectFileExtensionFromHeader(data);
            detectedMimeType = detectedExtension
              ? getMimeTypeFromExtension(detectedExtension)
              : "application/octet-stream";
          }
        } else {
          throw new Error("파일 데이터가 없습니다");
        }

        // Blob 생성
        const blob = new Blob([fileData], { type: detectedMimeType });
        const url = URL.createObjectURL(blob);
        setMediaUrl(url);
      } catch (err) {
        console.error("미디어 로드 실패:", err);
        setError(err instanceof Error ? err.message : "미디어 로드 실패");
      } finally {
        setLoading(false);
      }
    };

    loadMedia();

    // Cleanup
    return () => {
      if (mediaUrl) {
        URL.revokeObjectURL(mediaUrl);
      }
    };
  }, [filePath, data, mimeType]);

  if (loading) {
    return (
      <div className={className}>
        <div className="flex items-center justify-center h-full">
          <div className="text-gray-500">로딩 중...</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={className}>
        <div className="flex items-center justify-center h-full">
          <div className="text-red-500">오류: {error}</div>
        </div>
      </div>
    );
  }

  if (!mediaUrl) {
    return (
      <div className={className}>
        <div className="flex items-center justify-center h-full">
          <div className="text-gray-500">미디어를 로드할 수 없습니다</div>
        </div>
      </div>
    );
  }

  if (isImageDataType(dataType)) {
    return (
      <div className={className}>
        <img
          src={mediaUrl}
          alt="Image preview"
          className="max-w-full max-h-full object-contain"
          onLoad={() => URL.revokeObjectURL(mediaUrl)}
          onError={() => URL.revokeObjectURL(mediaUrl)}
        />
      </div>
    );
  }

  if (isVideoDataType(dataType)) {
    return (
      <div className={className}>
        <video
          src={mediaUrl}
          controls
          className="max-w-full max-h-full"
          onLoadedData={() => URL.revokeObjectURL(mediaUrl)}
          onError={() => URL.revokeObjectURL(mediaUrl)}
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
          src={mediaUrl}
          controls
          className="w-full"
          onLoadedData={() => URL.revokeObjectURL(mediaUrl)}
          onError={() => URL.revokeObjectURL(mediaUrl)}
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
