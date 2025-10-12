import { useState, useEffect } from 'react';
import Editor from '@monaco-editor/react';

import type { SessionStore } from '@/entities/session';
import { Button, Input, Badge, Tabs, TabsContent, TabsList, TabsTrigger } from '@/shared/ui';
import { useSessionEdit } from '../hooks/use-session-edit';
import { toast } from 'sonner';
import { Save, X } from 'lucide-react';

interface SessionEditorProps {
  session: SessionStore;
  isEditing: boolean;
  onSave: () => void;
  onCancel: () => void;
}

/**
 * 세션 편집을 위한 폼 기반 편집 컴포넌트
 */
export const SessionEditor = ({ session, isEditing, onSave, onCancel }: SessionEditorProps) => {
  const { form, saveChanges, startEditing } = useSessionEdit({ session });
  const [isSaving, setIsSaving] = useState(false);

  // 편집 모드가 활성화될 때 startEditing 호출
  useEffect(() => {
    if (isEditing) {
      startEditing();
    }
  }, [isEditing, startEditing]);

  const handleSave = async () => {
    try {
      setIsSaving(true);
      await saveChanges();
      toast.success('Session updated successfully');
      onSave();
    } catch (error) {
      toast.error('Failed to save session');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between p-4 border-b border-border bg-card">
        <div className="flex items-center gap-2">
          <h4 className="font-semibold text-card-foreground">{isEditing ? 'Edit Session' : 'Session Details'}</h4>
        </div>
        {isEditing && (
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={handleSave} disabled={isSaving}>
              <Save className="w-4 h-4" />
            </Button>
            <Button variant="ghost" size="sm" onClick={onCancel} disabled={isSaving}>
              <X className="w-4 h-4" />
            </Button>
          </div>
        )}
      </div>

      <div className="p-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Basic Info */}
          <div className="space-y-3">
            <div>
              <label className="text-xs font-medium text-muted-foreground">URL</label>
              <form.Field name="url">
                {(field) => (
                  <div className="mt-1 px-3 py-2 bg-muted/50 border border-input rounded-md text-sm text-muted-foreground break-all">
                    {field.state.value}
                  </div>
                )}
              </form.Field>
            </div>

            <div>
              <label className="text-xs font-medium text-muted-foreground">Method</label>
              <form.Field name="method">
                {(field) => (
                  <div className="mt-1">
                    <Badge variant="outline" className="text-xs font-mono">
                      {field.state.value}
                    </Badge>
                  </div>
                )}
              </form.Field>
            </div>

            <div>
              <label className="text-xs font-medium text-muted-foreground">Status</label>
              <form.Field name="isActive">
                {(field) => (
                  <div className="mt-1">
                    <Badge
                      variant={field.state.value ? 'default' : 'secondary'}
                      className={isEditing ? 'cursor-pointer hover:opacity-80' : 'cursor-default'}
                      onClick={() => isEditing && field.handleChange(!field.state.value)}
                    >
                      {field.state.value ? 'Active' : 'Inactive'}
                    </Badge>
                  </div>
                )}
              </form.Field>
            </div>
          </div>

          {/* Response Status */}
          <div className="space-y-3">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Response Status</label>
              <form.Field name="response.status">
                {(field) => (
                  <Input
                    type="number"
                    value={field.state.value || ''}
                    onChange={(e) => field.handleChange(parseInt(e.target.value) || 200)}
                    placeholder="200"
                    disabled={!isEditing}
                    className={`mt-1 ${isEditing ? '' : 'bg-muted/50 cursor-not-allowed'}`}
                  />
                )}
              </form.Field>
            </div>
          </div>
        </div>

        {/* Request/Response Tabs */}
        <Tabs defaultValue="request-headers" className="w-full">
          <TabsList className="grid w-full grid-cols-4 flex-shrink-0">
            <TabsTrigger value="request-headers">Request Headers</TabsTrigger>
            <TabsTrigger value="request-data">Request Data</TabsTrigger>
            <TabsTrigger value="response-headers">Response Headers</TabsTrigger>
            <TabsTrigger value="response-data">Response Data</TabsTrigger>
          </TabsList>

          <TabsContent value="request-headers" className="mt-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Request Headers</label>
              <form.Field name="request.headers">
                {(field) => (
                  <div className="mt-1 border rounded-md overflow-hidden">
                    <Editor
                      height="200px"
                      defaultLanguage="json" // TODO: 데이터 타입에 따라 동적으로 설정 (json, text, binary, image 등)
                      value={(() => {
                        // TODO 데이터 타입에 따라 바꿔야 할 부분
                        if (!field.state.value) return '';

                        // 이미 문자열인 경우 파싱 시도
                        if (typeof field.state.value === 'string') {
                          try {
                            const parsed = JSON.parse(field.state.value);
                            return JSON.stringify(parsed, null, 2);
                          } catch {
                            return field.state.value;
                          }
                        }

                        // 객체인 경우 그대로 문자열화
                        return JSON.stringify(field.state.value, null, 2);
                      })()}
                      onChange={(value) => {
                        if (isEditing && value !== undefined) {
                          // 빈 텍스트인 경우 undefined로 저장
                          if (value.trim() === '') {
                            field.handleChange(undefined as any);
                            return;
                          }

                          try {
                            const parsed = JSON.parse(value);
                            // 빈 객체인 경우 undefined로 저장
                            if (parsed && typeof parsed === 'object' && Object.keys(parsed).length === 0) {
                              field.handleChange(undefined as any);
                            } else {
                              field.handleChange(parsed);
                            }
                          } catch {
                            // Invalid JSON, save as string
                            field.handleChange(value as any);
                          }
                        }
                      }}
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        fontSize: 12,
                        lineNumbers: 'on',
                        roundedSelection: false,
                        scrollbar: {
                          vertical: 'auto',
                          horizontal: 'auto',
                        },
                        automaticLayout: true,
                        formatOnPaste: isEditing,
                        formatOnType: isEditing,
                        readOnly: !isEditing,
                      }}
                      theme="vs"
                    />
                  </div>
                )}
              </form.Field>
            </div>
          </TabsContent>

          <TabsContent value="request-data" className="mt-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Request Data</label>
              <form.Field name="request.data">
                {(field) => (
                  <div className="mt-1 border rounded-md overflow-hidden">
                    <Editor
                      height="200px"
                      defaultLanguage="json" // TODO: 데이터 타입에 따라 동적으로 설정 (json, text, binary, image 등)
                      value={(() => {
                        // TODO 데이터 타입에 따라 바꿔야 할 부분
                        if (!field.state.value) return '';

                        if (typeof field.state.value === 'string') {
                          try {
                            const parsed = JSON.parse(field.state.value);
                            return JSON.stringify(parsed, null, 2);
                          } catch {
                            return field.state.value;
                          }
                        }

                        return JSON.stringify(field.state.value, null, 2);
                      })()}
                      onChange={(value) => {
                        if (isEditing && value !== undefined) {
                          // 빈 텍스트인 경우 undefined로 저장
                          if (value.trim() === '') {
                            field.handleChange(undefined as any);
                            return;
                          }

                          try {
                            const parsed = JSON.parse(value);
                            // 빈 객체인 경우 undefined로 저장
                            if (parsed && typeof parsed === 'object' && Object.keys(parsed).length === 0) {
                              field.handleChange(undefined as any);
                            } else {
                              field.handleChange(parsed);
                            }
                          } catch {
                            // Invalid JSON, save as string
                            field.handleChange(value as any);
                          }
                        }
                      }}
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        fontSize: 12,
                        lineNumbers: 'on',
                        roundedSelection: false,
                        scrollbar: {
                          vertical: 'auto',
                          horizontal: 'auto',
                        },
                        automaticLayout: true,
                        formatOnPaste: isEditing,
                        formatOnType: isEditing,
                        readOnly: !isEditing,
                      }}
                      theme="vs"
                    />
                  </div>
                )}
              </form.Field>
            </div>
          </TabsContent>

          <TabsContent value="response-headers" className="mt-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Response Headers</label>
              <form.Field name="response.headers">
                {(field) => (
                  <div className="mt-1 border rounded-md overflow-hidden">
                    <Editor
                      height="200px"
                      defaultLanguage="json" // TODO: 데이터 타입에 따라 동적으로 설정 (json, text, binary, image 등)
                      value={(() => {
                        // TODO 데이터 타입에 따라 바꿔야 할 부분
                        if (!field.state.value) return '';

                        // 이미 문자열인 경우 파싱 시도
                        if (typeof field.state.value === 'string') {
                          try {
                            const parsed = JSON.parse(field.state.value);
                            return JSON.stringify(parsed, null, 2);
                          } catch {
                            return field.state.value;
                          }
                        }

                        // 객체인 경우 그대로 문자열화
                        return JSON.stringify(field.state.value, null, 2);
                      })()}
                      onChange={(value) => {
                        if (isEditing && value !== undefined) {
                          // 빈 텍스트인 경우 undefined로 저장
                          if (value.trim() === '') {
                            field.handleChange(undefined as any);
                            return;
                          }

                          try {
                            const parsed = JSON.parse(value);
                            // 빈 객체인 경우 undefined로 저장
                            if (parsed && typeof parsed === 'object' && Object.keys(parsed).length === 0) {
                              field.handleChange(undefined as any);
                            } else {
                              field.handleChange(parsed);
                            }
                          } catch {
                            // Invalid JSON, save as string
                            field.handleChange(value as any);
                          }
                        }
                      }}
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        fontSize: 12,
                        lineNumbers: 'on',
                        roundedSelection: false,
                        scrollbar: {
                          vertical: 'auto',
                          horizontal: 'auto',
                        },
                        automaticLayout: true,
                        formatOnPaste: isEditing,
                        formatOnType: isEditing,
                        readOnly: !isEditing,
                      }}
                      theme="vs"
                    />
                  </div>
                )}
              </form.Field>
            </div>
          </TabsContent>

          <TabsContent value="response-data" className="mt-4">
            <div>
              <label className="text-xs font-medium text-muted-foreground">Response Data</label>
              <form.Field name="response.data">
                {(field) => (
                  <div className="mt-1 border rounded-md overflow-hidden">
                    <Editor
                      height="200px"
                      defaultLanguage="json" // TODO: 데이터 타입에 따라 동적으로 설정 (json, text, binary, image 등)
                      value={(() => {
                        // TODO 데이터 타입에 따라 바꿔야 할 부분
                        if (!field.state.value) return '';

                        // 이미 문자열인 경우 파싱 시도
                        if (typeof field.state.value === 'string') {
                          try {
                            const parsed = JSON.parse(field.state.value);
                            return JSON.stringify(parsed, null, 2);
                          } catch {
                            return field.state.value;
                          }
                        }

                        // 객체인 경우 그대로 문자열화
                        return JSON.stringify(field.state.value, null, 2);
                      })()}
                      onChange={(value) => {
                        if (isEditing && value !== undefined) {
                          // 빈 텍스트인 경우 undefined로 저장
                          if (value.trim() === '') {
                            field.handleChange(undefined as any);
                            return;
                          }

                          try {
                            const parsed = JSON.parse(value);
                            // 빈 객체인 경우 undefined로 저장
                            if (parsed && typeof parsed === 'object' && Object.keys(parsed).length === 0) {
                              field.handleChange(undefined as any);
                            } else {
                              field.handleChange(parsed);
                            }
                          } catch {
                            // Invalid JSON, save as string
                            field.handleChange(value as any);
                          }
                        }
                      }}
                      options={{
                        minimap: { enabled: false },
                        scrollBeyondLastLine: false,
                        fontSize: 12,
                        lineNumbers: 'on',
                        roundedSelection: false,
                        scrollbar: {
                          vertical: 'auto',
                          horizontal: 'auto',
                        },
                        automaticLayout: true,
                        formatOnPaste: isEditing,
                        formatOnType: isEditing,
                        readOnly: !isEditing,
                      }}
                      theme="vs"
                    />
                  </div>
                )}
              </form.Field>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
};
