import { Copy } from 'lucide-react';
import { writeImage, writeText } from '@tauri-apps/plugin-clipboard-manager';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';
import { Editor } from '@monaco-editor/react';

import { getBodyForDisplay, createImageDataUrl } from '../lib/utils';
import { dataTypeToMonacoLanguage, isImageDataType } from '@/entities/proxy/model/data-type';
import { toast } from 'sonner';
import { ImagePreview } from './image-preview';

interface TransactionResponseProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionResponse = ({ transaction, isEditing = false, form }: TransactionResponseProps) => {
  const { response } = transaction;

  if (!response) return null;

  const getResponseText = () => {
    return getBodyForDisplay(response.body, response.data_type, response.body_json);
  };

  const responseText = getResponseText();

  const handleCopy = async () => {
    if (isImageDataType(response.data_type)) {
      try {
        // Tauri 클립보드 매니저를 사용하여 이미지 복사
        await writeImage(response.body);
        toast.success('Image copied to clipboard');
      } catch (error) {
        console.error('Failed to copy image:', error);
        // Tauri 클립보드 실패 시 다운로드로 fallback
        try {
          const dataUrl = createImageDataUrl(response.body, response.data_type);
          if (dataUrl) {
            const link = document.createElement('a');
            link.href = dataUrl;
            link.download = `image.${getImageFileExtension(response.data_type)}`;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            toast.success('Image downloaded (clipboard failed)');
          } else {
            toast.error('Failed to copy or download image');
          }
        } catch (fallbackError) {
          console.error('Fallback download failed:', fallbackError);
          toast.error('Failed to copy or download image');
        }
      }
    } else {
      try {
        // Tauri 클립보드 매니저를 사용하여 텍스트 복사
        await writeText(responseText);
        toast.success('Response body copied to clipboard');
      } catch (error) {
        console.error('Failed to copy text:', error);
        toast.error('Failed to copy to clipboard');
      }
    }
  };

  const getImageFileExtension = (dataType: string): string => {
    // MIME 타입에서 확장자 추출하는 간단한 함수
    return 'png'; // 기본값
  };

  return (
    <Card className="gap-0 flex flex-col min-h-0 flex-1">
      <CardHeader className="flex-shrink-0">
        <div className="flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 p-0 min-h-0">
        {isImageDataType(response.data_type) ? (
          <div className="h-[calc(100vh-300px)] border rounded-md overflow-auto p-4">
            <ImagePreview data={response.body} dataType={response.data_type} className="h-full" />
          </div>
        ) : form && isEditing ? (
          <form.Field
            name="response.data"
            children={(field) => (
              <div className="h-[calc(100vh-300px)] border rounded-md overflow-hidden">
                <Editor
                  height="calc(100vh - 300px)"
                  language={dataTypeToMonacoLanguage(response.data_type)}
                  value={(field.state.value as string) || ''}
                  onChange={(value) => field.handleChange(value || '')}
                  options={{
                    minimap: { enabled: false },
                    scrollBeyondLastLine: false,
                    fontSize: 12,
                    lineNumbers: 'on',
                    wordWrap: 'on',
                    automaticLayout: true,
                    padding: { top: 8, bottom: 8 },
                    scrollbar: {
                      vertical: 'auto',
                      horizontal: 'auto',
                    },
                  }}
                />
              </div>
            )}
          />
        ) : (
          <div className="h-[calc(100vh-300px)] border rounded-md overflow-hidden">
            <Editor
              height="calc(100vh - 300px)"
              language={dataTypeToMonacoLanguage(response.data_type)}
              value={responseText}
              options={{
                readOnly: true,
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                fontSize: 12,
                lineNumbers: 'on',
                wordWrap: 'on',
                automaticLayout: true,
                padding: { top: 8, bottom: 8 },
                scrollbar: {
                  vertical: 'auto',
                  horizontal: 'auto',
                },
              }}
            />
          </div>
        )}
      </CardContent>
    </Card>
  );
};
