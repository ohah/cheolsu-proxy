import { Copy } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';
import { Editor } from '@monaco-editor/react';

import { formatBody, detectContentType } from '../lib';

interface TransactionResponseProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionResponse = ({ transaction, isEditing = false, form }: TransactionResponseProps) => {
  const { response } = transaction;

  if (!response) return null;

  const getResponseText = () => {
    return formatBody(response.body);
  };

  const responseText = getResponseText();
  const contentType = detectContentType(responseText);

  const handleCopy = () => {
    navigator.clipboard.writeText(responseText);
  };

  return (
    <Card className="gap-0 flex flex-col min-h-0 flex-1">
      <CardHeader className="flex-shrink-0">
        <div className="flex items-center justify-end">
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 p-0 min-h-0">
        {form && isEditing ? (
          <form.Field
            name="response.data"
            children={(field) => (
              <div className="h-[calc(100vh-300px)] border rounded-md overflow-hidden">
                <Editor
                  height="calc(100vh - 300px)"
                  defaultLanguage={contentType}
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
              defaultLanguage={contentType}
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
