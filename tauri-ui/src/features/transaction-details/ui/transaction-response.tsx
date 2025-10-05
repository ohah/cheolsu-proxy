import { Editor } from '@monaco-editor/react';
import { HttpResponse } from '@/entities/proxy/model/types';
import { dataTypeToMonacoLanguage } from '@/entities/proxy/model/data-type';
import { getBodyForDisplay } from '../lib/utils';

interface TransactionResponseProps {
  response?: HttpResponse;
  isEditing?: boolean;
  form?: any;
}

export const TransactionResponse = ({ response, isEditing, form }: TransactionResponseProps) => {
  // TODO: isEditing과 form을 사용한 편집 기능 구현
  console.log('TransactionResponse props:', { isEditing, form });
  // response가 없으면 빈 컴포넌트 반환
  if (!response) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold">Response Body</h3>
          <span className="text-sm text-muted-foreground">No response data</span>
        </div>
        <div className="border rounded-lg p-4 text-center text-muted-foreground">Response data is not available</div>
      </div>
    );
  }

  const bodyContent = getBodyForDisplay(response.body, response.data_type, response.body_json);
  const language = dataTypeToMonacoLanguage(response.data_type);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Response Body</h3>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">
            Data Type: <span className="font-mono bg-green-100 px-2 py-1 rounded">{response.data_type}</span> •{' '}
            {(response.body || new Uint8Array()).length} bytes
          </span>
        </div>
      </div>

      {/* 디버깅 정보 */}
      <div className="text-xs text-gray-500 bg-gray-50 p-2 rounded">
        <div>Status: {response.status || 'N/A'}</div>
        <div>Version: {response.version || 'N/A'}</div>
        <div>Body exists: {response.body ? 'Yes' : 'No'}</div>
        <div>Body length: {(response.body || new Uint8Array()).length}</div>
        <div>Data type: {response.data_type || 'N/A'}</div>
      </div>

      <div className="border rounded-lg overflow-hidden">
        <Editor
          height="400px"
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
