import { useState } from 'react';
import { Download, Maximize2, Minimize2 } from 'lucide-react';

import { Button } from '@/shared/ui';
import { createImageDataUrl } from '../lib/utils';
import type { DataType } from '@/entities/proxy/model/types';

interface ImagePreviewProps {
  data: Uint8Array | number[];
  dataType: DataType;
  className?: string;
}

export const ImagePreview = ({ data, dataType, className = '' }: ImagePreviewProps) => {
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [imageError, setImageError] = useState(false);

  const dataUrl = createImageDataUrl(data, dataType);

  if (!dataUrl || imageError) {
    return (
      <div className={`flex items-center justify-center p-8 bg-gray-50 rounded-md ${className}`}>
        <div className="text-center">
          <p className="text-gray-500 mb-2">이미지를 표시할 수 없습니다</p>
          <p className="text-sm text-gray-400">
            {dataType} 파일 ({data.length} bytes)
          </p>
        </div>
      </div>
    );
  }

  const handleDownload = () => {
    const link = document.createElement('a');
    link.href = dataUrl;
    link.download = `image.${getFileExtension(dataType)}`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const getFileExtension = (type: DataType): string => {
    switch (type) {
      case 'Image':
        // MIME 타입에서 확장자 추출
        const mimeType = dataUrl.split(';')[0].split(':')[1];
        switch (mimeType) {
          case 'image/png':
            return 'png';
          case 'image/jpeg':
            return 'jpg';
          case 'image/gif':
            return 'gif';
          case 'image/webp':
            return 'webp';
          case 'image/svg+xml':
            return 'svg';
          default:
            return 'png';
        }
      default:
        return 'bin';
    }
  };

  const imageElement = (
    <div className="relative group">
      <img
        src={dataUrl}
        alt="이미지 미리보기"
        className={`max-w-full h-auto rounded-md shadow-sm ${isFullscreen ? 'max-h-none' : 'max-h-[400px]'}`}
        onError={() => setImageError(true)}
        style={{
          maxWidth: isFullscreen ? '90vw' : '100%',
          maxHeight: isFullscreen ? '90vh' : '400px',
        }}
      />

      {/* 호버 시 컨트롤 버튼들 */}
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <div className="flex gap-1">
          <Button variant="secondary" size="sm" onClick={() => setIsFullscreen(!isFullscreen)} className="h-8 w-8 p-0">
            {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
          </Button>
          <Button variant="secondary" size="sm" onClick={handleDownload} className="h-8 w-8 p-0">
            <Download className="w-4 h-4" />
          </Button>
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
      <div className="text-sm text-gray-500">
        {dataType} 파일 ({data.length} bytes)
      </div>
    </div>
  );
};
