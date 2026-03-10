import { useState, useEffect } from "react";
import { Download, Maximize2, Minimize2 } from "lucide-react";
import { Trans } from "@lingui/react/macro";

import { Button, Tooltip, TooltipTrigger, TooltipContent } from "@/shared/ui";
import { createImageDataUrl } from "../lib/utils";
import { useBase64Worker } from "@/hooks/use-base64-worker";
import type { DataType } from "@/entities/proxy/model/types";

interface ImagePreviewProps {
  data: Uint8Array | number[];
  dataType: DataType;
  className?: string;
}

export const ImagePreview = ({ data, dataType, className = "" }: ImagePreviewProps) => {
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [imageError, setImageError] = useState(false);
  const [dataUrl, setDataUrl] = useState<string>("");
  const [isLoading, setIsLoading] = useState(true);

  const { encodeToBase64, isWorkerAvailable } = useBase64Worker();

  useEffect(() => {
    if (dataType === "Image") {
      setIsLoading(true);
      setImageError(false);

      // Worker가 사용 가능한 경우 Worker 사용, 그렇지 않으면 기존 방식 사용
      if (isWorkerAvailable()) {
        encodeToBase64(data, dataType)
          .then(setDataUrl)
          .catch(() => {
            // Worker 실패 시 기존 방식으로 fallback
            const fallbackDataUrl = createImageDataUrl(data, dataType);
            if (fallbackDataUrl) {
              setDataUrl(fallbackDataUrl);
            } else {
              setImageError(true);
            }
          })
          .finally(() => {
            setIsLoading(false);
          });
      } else {
        // Worker를 사용할 수 없는 경우 기존 방식 사용
        const fallbackDataUrl = createImageDataUrl(data, dataType);
        if (fallbackDataUrl) {
          setDataUrl(fallbackDataUrl);
        } else {
          setImageError(true);
        }
        setIsLoading(false);
      }
    } else {
      setIsLoading(false);
    }
  }, [data, dataType, encodeToBase64, isWorkerAvailable]);

  if (isLoading) {
    return (
      <div className={`flex items-center justify-center p-8 bg-muted rounded-md ${className}`}>
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-foreground mx-auto mb-2"></div>
          <p className="text-muted-foreground">이미지 로딩 중...</p>
          <p className="text-xs text-muted-foreground/70 mt-1">
            {data.length > 1024 * 1024
              ? `${Math.round(data.length / 1024 / 1024)}MB`
              : `${Math.round(data.length / 1024)}KB`}
          </p>
        </div>
      </div>
    );
  }

  if (!dataUrl || imageError) {
    return (
      <div className={`flex items-center justify-center p-8 bg-muted rounded-md ${className}`}>
        <div className="text-center">
          <p className="text-muted-foreground mb-2">이미지를 표시할 수 없습니다</p>
          <p className="text-sm text-muted-foreground/70">
            {dataType} 파일 ({data.length} bytes)
          </p>
        </div>
      </div>
    );
  }

  const handleDownload = () => {
    const link = document.createElement("a");
    link.href = dataUrl;
    link.download = `image.${getFileExtension(dataType)}`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const getFileExtension = (type: DataType): string => {
    switch (type) {
      case "Image":
        // MIME 타입에서 확장자 추출
        const mimeType = dataUrl.split(";")[0].split(":")[1];
        switch (mimeType) {
          case "image/png":
            return "png";
          case "image/jpeg":
            return "jpg";
          case "image/gif":
            return "gif";
          case "image/webp":
            return "webp";
          case "image/svg+xml":
            return "svg";
          default:
            return "png";
        }
      default:
        return "bin";
    }
  };

  const imageElement = (
    <div className="relative group">
      <img
        src={dataUrl}
        alt="이미지 미리보기"
        className={`max-w-full h-auto rounded-md shadow-sm ${isFullscreen ? "max-h-none" : "max-h-[400px]"}`}
        onError={() => setImageError(true)}
        style={{
          maxWidth: isFullscreen ? "90vw" : "100%",
          maxHeight: isFullscreen ? "90vh" : "400px",
        }}
      />

      {/* 호버 시 컨트롤 버튼들 */}
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <div className="flex gap-1">
          <Tooltip>
            <TooltipTrigger render={<div />}>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setIsFullscreen(!isFullscreen)}
                className="h-8 w-8 p-0"
              >
                {isFullscreen ? (
                  <Minimize2 className="w-4 h-4" />
                ) : (
                  <Maximize2 className="w-4 h-4" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent sideOffset={4}>
              {isFullscreen ? <Trans>Exit fullscreen</Trans> : <Trans>Fullscreen</Trans>}
            </TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger render={<div />}>
              <Button
                variant="secondary"
                size="sm"
                onClick={handleDownload}
                className="h-8 w-8 p-0"
              >
                <Download className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent sideOffset={4}>
              <Trans>Download</Trans>
            </TooltipContent>
          </Tooltip>
        </div>
      </div>
    </div>
  );

  if (isFullscreen) {
    return (
      <div className="fixed inset-0 z-50 bg-black bg-opacity-90 flex items-center justify-center p-4">
        <div className="relative max-w-full max-h-full">
          {imageElement}
          <Button
            variant="secondary"
            size="sm"
            onClick={() => setIsFullscreen(false)}
            className="absolute top-4 left-4"
          >
            닫기
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col items-center gap-4 ${className}`}>
      {imageElement}
      <div className="text-sm text-muted-foreground">
        {dataType} 파일 ({data.length} bytes)
      </div>
    </div>
  );
};
