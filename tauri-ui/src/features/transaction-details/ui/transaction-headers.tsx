import { Copy, Plus, Trash2 } from 'lucide-react';

import type { HttpTransaction } from '@/entities/proxy';

import { Button, Card, CardContent, CardHeader, Input } from '@/shared/ui';
import type { AppFormInstance } from '../context/form-context';
import { toast } from 'sonner';

interface TransactionHeadersProps {
  transaction: HttpTransaction;
  isEditing?: boolean;
  form?: AppFormInstance;
}

export const TransactionHeaders = ({ transaction, isEditing = false, form }: TransactionHeadersProps) => {
  const { request } = transaction;

  if (!request?.headers) return null;

  const handleCopy = () => {
    const headers = form?.getFieldValue('request.headers') || request.headers;
    const headersText = Object.entries(headers)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n');
    navigator.clipboard.writeText(headersText);
    toast.success('Request headers copied to clipboard');
  };

  const handleAddHeader = () => {
    if (!form) return;
    const currentHeaders = form.getFieldValue('request.headers') || {};
    const newHeaders = { ...currentHeaders, '': '' };
    form.setFieldValue('request.headers', newHeaders);
  };

  const handleRemoveHeader = (key: string) => {
    if (!form) return;
    const currentHeaders = form.getFieldValue('request.headers') || {};
    const newHeaders = { ...currentHeaders } as Record<string, string>;
    delete newHeaders[key];
    form.setFieldValue('request.headers', newHeaders);
  };

  return (
    <Card className="gap-0">
      <CardHeader>
        <div className="flex items-center justify-end gap-2">
          {isEditing && (
            <Button variant="ghost" size="sm" onClick={handleAddHeader}>
              <Plus className="w-4 h-4" />
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="w-4 h-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {form && isEditing ? (
            <form.Field
              name="request.headers"
              children={(field: any) => (
                <>
                  {Object.entries(field.state.value || {}).map(([key, value], index) => (
                    <div key={`header-${index}`} className="flex items-center gap-2 text-sm">
                      <Input
                        value={key}
                        onChange={(e) => {
                          const newHeaders = { ...field.state.value };
                          delete newHeaders[key];
                          newHeaders[e.target.value] = value;
                          field.handleChange(newHeaders);
                        }}
                        placeholder="Header name"
                        className="font-mono text-xs flex-1"
                      />
                      <Input
                        value={value as string}
                        onChange={(e) => {
                          const newHeaders = { ...field.state.value };
                          newHeaders[key] = e.target.value;
                          field.handleChange(newHeaders);
                        }}
                        placeholder="Header value"
                        className="font-mono text-xs flex-2"
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRemoveHeader(key)}
                        className="h-9 w-9 p-0 flex-shrink-0"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  ))}
                </>
              )}
            />
          ) : (
            Object.entries(request.headers).map(([key, value]) => (
              <div key={key} className="flex items-center gap-2 text-sm">
                <span className="text-muted-foreground font-mono flex-1">{key}:</span>
                <span className="font-mono break-all flex-2">{value}</span>
              </div>
            ))
          )}
        </div>
      </CardContent>
    </Card>
  );
};
