import { Editor } from '@monaco-editor/react';
import { HttpRequest } from '@/entities/proxy/model/types';
import { dataTypeToMonacoLanguage } from '@/entities/proxy/model/data-type';
import { getBodyForDisplay } from '../lib/utils';

interface TransactionBodyProps {
  request?: HttpRequest;
  isEditing?: boolean;
  form?: any;
}

export const TransactionBody = ({ request, isEditing, form }: TransactionBodyProps) => {
  // TODO: isEditing과 form을 사용한 편집 기능 구현
  console.log('TransactionBody props:', { isEditing, form });
  // request가 없으면 빈 컴포넌트 반환
  if (!request) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold">Request Body</h3>
          <span className="text-sm text-muted-foreground">No request data</span>
        </div>
        <div className="border rounded-lg p-4 text-center text-muted-foreground">Request data is not available</div>
      </div>
    );
  }

  const bodyContent = getBodyForDisplay(request.body || new Uint8Array(), request.data_type, request.body_json);
  const language = dataTypeToMonacoLanguage(request.data_type);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Request Body</h3>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">
            Data Type: <span className="font-mono bg-blue-100 px-2 py-1 rounded">{request.data_type}</span> •{' '}
            {(request.body || new Uint8Array()).length} bytes
          </span>
        </div>
      </div>

      {/* 디버깅 정보 */}
      <div className="text-xs text-gray-500 bg-gray-50 p-2 rounded">
        <div>Request ID: {request.id || 'N/A'}</div>
        <div>Method: {request.method || 'N/A'}</div>
        <div>URI: {request.uri || 'N/A'}</div>
        <div>Body exists: {request.body ? 'Yes' : 'No'}</div>
        <div>Body length: {(request.body || new Uint8Array()).length}</div>
        <div>Data type: {request.data_type || 'N/A'}</div>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <Editor
          height="300px"
          language={language}
          value={bodyContent}
          options={{
            readOnly: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            wordWrap: 'on',
            fontSize: 14,
            lineNumbers: 'on',
            folding: true,
            lineDecorationsWidth: 0,
            lineNumbersMinChars: 0,
            renderLineHighlight: 'none',
            overviewRulerBorder: false,
            hideCursorInOverviewRuler: true,
            overviewRulerLanes: 0,
            scrollbar: {
              vertical: 'auto',
              horizontal: 'auto',
              verticalScrollbarSize: 8,
              horizontalScrollbarSize: 8,
            },
          }}
        />
      </div>
    </div>
  );
};
