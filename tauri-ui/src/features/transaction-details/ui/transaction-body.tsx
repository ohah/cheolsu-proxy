import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';
import { Editor } from '@monaco-editor/react';

import { formatBody, detectContentType, contentTypeToMonacoLanguage } from '../lib';
import { toast } from 'sonner';

interface TransactionBodyProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionBody = ({ transaction, isEditing = false, form }: TransactionBodyProps) => {
  const { request } = transaction;

  if (!request) return null;

  const getRequestText = () => {
    if (!request?.body || request.body.length === 0) {
      return '';
    }
    return formatBody(request.body);
  };

  const requestText = getRequestText();

  // Rust에서 전달받은 content_type을 우선 사용하고, 없으면 기존 방식으로 감지
  const detectedContentType = request.content_type || detectContentType(requestText);

  const handleCopy = () => {
    navigator.clipboard.writeText(requestText);
    toast.success('Request body copied to clipboard');
  };

  return (
    <Card className="gap-0 flex flex-col min-h-0 flex-1">
      <CardHeader className="flex-shrink-0">
        <div className="flex items-center justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={handleCopy} title="요청 Body 내용을 클립보드에 복사">
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 p-0 min-h-0">
        {form && isEditing ? (
          <form.Field
            name="request.data"
            children={(field) => (
              <div className="h-[calc(100vh-300px)] border rounded-md overflow-hidden">
                <Editor
                  height="calc(100vh - 300px)"
                  language={contentTypeToMonacoLanguage(detectedContentType)}
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
              language={contentTypeToMonacoLanguage(detectedContentType)}
              value={requestText}
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
