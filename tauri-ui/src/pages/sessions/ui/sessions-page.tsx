import { useState } from 'react';

import { useSessionStore, useProxyStore } from '@/shared/stores';
import { Card, CardContent, CardHeader } from '@/shared/ui';
import { Badge } from '@/shared/ui';
import { Trash2, Edit } from 'lucide-react';
import { Button } from '@/shared/ui';
import { toast } from 'sonner';
import { AppSidebar } from '@/shared/app-sidebar';
import { SessionEditor } from './session-editor';

/**
 * 세션 데이터를 표시하는 페이지
 * useSessionStore에 저장된 세션 정보를 테이블 형태로 보여줍니다.
 * NetworkDashboard와 동일한 레이아웃을 사용합니다.
 */
export const SessionsPage = () => {
  const { isConnected } = useProxyStore();
  const { sessions, deleteSession } = useSessionStore();
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);

  const handleDeleteSession = (id: string) => {
    deleteSession(id);
    toast.success('Session deleted successfully');
  };

  const handleEditSession = (sessionId: string) => {
    setEditingSessionId(sessionId);
  };

  const handleSaveSession = () => {
    setEditingSessionId(null);
  };

  const handleCancelEdit = () => {
    setEditingSessionId(null);
  };

  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar isConnected={isConnected} />

      <div className="flex-1 flex flex-col h-full">
        {/* Header similar to NetworkHeader */}
        <div className="flex items-center justify-between p-4 border-b border-border bg-background">
          <div className="flex items-center gap-2">
            <h1 className="font-semibold text-card-foreground">Saved Sessions</h1>
            <Badge variant="outline" className="text-xs">
              {sessions.length} sessions
            </Badge>
          </div>
        </div>

        <div className="flex-1 overflow-auto p-4">
          {sessions.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <div className="text-center space-y-2">
                  <h3 className="text-lg font-semibold">No sessions found</h3>
                  <p className="text-muted-foreground">Start making HTTP requests to see your sessions here.</p>
                </div>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-4">
              {sessions.map((session) => (
                <Card key={session.id} className="overflow-hidden">
                  <CardHeader className="pb-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="flex items-center gap-2">
                          <Badge variant="outline" className="text-xs font-mono">
                            {session.method}
                          </Badge>
                          <span className="text-sm font-medium text-card-foreground truncate max-w-md">
                            {session.url}
                          </span>
                        </div>
                        <Badge variant={session.isActive ? 'default' : 'secondary'} className="text-xs">
                          {session.isActive ? 'Active' : 'Inactive'}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleEditSession(session.id)}
                          title="Edit session"
                          disabled={editingSessionId === session.id}
                        >
                          <Edit className="w-4 h-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleDeleteSession(session.id)}
                          title="Delete session"
                          className="text-destructive hover:text-destructive"
                        >
                          <Trash2 className="w-4 h-4" />
                        </Button>
                      </div>
                    </div>
                  </CardHeader>
                  <CardContent className="pt-0">
                    <SessionEditor
                      session={session}
                      isEditing={editingSessionId === session.id}
                      onSave={handleSaveSession}
                      onCancel={handleCancelEdit}
                    />
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
