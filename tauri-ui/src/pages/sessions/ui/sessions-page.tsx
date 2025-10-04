import { useSessionStore } from '@/shared/stores';
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui';
import { Badge } from '@/shared/ui';
import { Trash2, Copy, ExternalLink } from 'lucide-react';
import { Button } from '@/shared/ui';
import { toast } from 'sonner';
import { AppSidebar } from '@/shared/app-sidebar';
import { useProxyInitialization } from '@/pages/network-dashboard/hooks';

/**
 * 세션 데이터를 표시하는 페이지
 * useSessionStore에 저장된 세션 정보를 테이블 형태로 보여줍니다.
 * NetworkDashboard와 동일한 레이아웃을 사용합니다.
 */
export const SessionsPage = () => {
  const { isConnected } = useProxyInitialization();
  const { sessions, deleteSession } = useSessionStore();

  const handleDeleteSession = (id: string) => {
    deleteSession(id);
    toast.success('Session deleted successfully');
  };

  const handleCopySession = (session: any) => {
    const sessionText = JSON.stringify(session, null, 2);
    navigator.clipboard.writeText(sessionText);
    toast.success('Session data copied to clipboard');
  };

  const handleOpenUrl = (url: string) => {
    window.open(url, '_blank');
  };

  return (
    <div className="flex h-[100vh] w-full">
      <AppSidebar isConnected={isConnected} />

      <div className="flex-1 flex flex-col h-full">
        <div className="p-6 space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">Saved Sessions</h1>
              <p className="text-muted-foreground">Manage and view your saved HTTP sessions</p>
            </div>
            <Badge variant="outline" className="text-sm">
              {sessions.length} sessions
            </Badge>
          </div>

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
                        <CardTitle className="text-lg font-semibold truncate max-w-md">{session.url}</CardTitle>
                        <Badge variant={session.isActive ? 'default' : 'secondary'} className="text-xs">
                          {session.isActive ? 'Active' : 'Inactive'}
                        </Badge>
                        <Badge variant="outline" className="text-xs">
                          {session.method}
                        </Badge>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleOpenUrl(session.url)}
                          title="Open URL in browser"
                        >
                          <ExternalLink className="w-4 h-4" />
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleCopySession(session)}
                          title="Copy session data"
                        >
                          <Copy className="w-4 h-4" />
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
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      {/* Request Section */}
                      <div className="space-y-2">
                        <h4 className="font-medium text-sm text-muted-foreground">Request</h4>
                        <div className="bg-muted/50 rounded-md p-3 space-y-2">
                          {session.request?.headers && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Headers:</span>
                              <pre className="text-xs mt-1 overflow-x-auto">
                                {JSON.stringify(session.request.headers, null, 2)}
                              </pre>
                            </div>
                          )}
                          {session.request?.data && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Data:</span>
                              <pre className="text-xs mt-1 overflow-x-auto">
                                {JSON.stringify(session.request.data, null, 2)}
                              </pre>
                            </div>
                          )}
                          {session.request?.params && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Params:</span>
                              <pre className="text-xs mt-1 overflow-x-auto">
                                {JSON.stringify(session.request.params, null, 2)}
                              </pre>
                            </div>
                          )}
                        </div>
                      </div>

                      {/* Response Section */}
                      <div className="space-y-2">
                        <h4 className="font-medium text-sm text-muted-foreground">Response</h4>
                        <div className="bg-muted/50 rounded-md p-3 space-y-2">
                          {session.response?.status && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Status:</span>
                              <Badge variant="outline" className="ml-2 text-xs">
                                {session.response.status}
                              </Badge>
                            </div>
                          )}
                          {session.response?.headers && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Headers:</span>
                              <pre className="text-xs mt-1 overflow-x-auto">
                                {JSON.stringify(session.response.headers, null, 2)}
                              </pre>
                            </div>
                          )}
                          {session.response?.data && (
                            <div>
                              <span className="text-xs font-medium text-muted-foreground">Data:</span>
                              <pre className="text-xs mt-1 overflow-x-auto">
                                {JSON.stringify(session.response.data, null, 2)}
                              </pre>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
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
