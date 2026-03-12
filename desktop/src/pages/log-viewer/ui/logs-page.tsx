import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  Button,
  Badge,
  Input,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "@/shared/ui";
import { formatBytes } from "@/shared/lib/format-bytes";
import { RefreshCw, Trash2, FolderOpen, Search, X, Shield, FileX2 } from "lucide-react";
import { toast } from "sonner";

interface LogFileInfo {
  name: string;
  path: string;
  size: number;
  modified: number;
}

interface TlsPassthroughEntry {
  host: string;
  failure_count: number;
}

function formatDate(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString();
}

function getLogLevelClass(line: string): string {
  if (line.includes("ERROR")) return "text-red-500";
  if (line.includes("WARN")) return "text-yellow-500";
  if (line.includes("DEBUG")) return "text-muted-foreground";
  if (line.includes("TRACE")) return "text-muted-foreground";
  return "";
}

type TabType = "logs" | "tls-passthrough";

export function LogsPage() {
  const { t } = useLingui();
  const [activeTab, setActiveTab] = useState<TabType>("logs");
  const [logFiles, setLogFiles] = useState<LogFileInfo[]>([]);
  const [selectedFile, setSelectedFile] = useState<LogFileInfo | null>(null);
  const [logContent, setLogContent] = useState("");
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(false);
  const contentRef = useRef<HTMLPreElement>(null);

  // Delete confirmation dialog state
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  // TLS Passthrough state
  const [tlsEntries, setTlsEntries] = useState<TlsPassthroughEntry[]>([]);
  const [tlsLoading, setTlsLoading] = useState(false);

  const selectedFileRef = useRef(selectedFile);
  selectedFileRef.current = selectedFile;

  const fetchLogFiles = useCallback(async () => {
    try {
      const files = await invoke<LogFileInfo[]>("get_log_files");
      setLogFiles(files);
      const current = selectedFileRef.current;
      // Auto-select first file if none selected
      if (!current && files.length > 0) {
        setSelectedFile(files[0]);
      }
      // Update selected file info if it still exists
      if (current) {
        const updated = files.find((f) => f.path === current.path);
        if (updated) {
          setSelectedFile(updated);
        }
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const fetchLogContent = useCallback(async () => {
    if (!selectedFile) return;
    try {
      const content = await invoke<string>("read_log_file", {
        path: selectedFile.path,
        tailLines: 1000,
      });
      setLogContent(content);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, [selectedFile]);

  const handleRefresh = useCallback(async () => {
    setLoading(true);
    await fetchLogFiles();
    await fetchLogContent();
    setLoading(false);
  }, [fetchLogFiles, fetchLogContent]);

  const handleClear = useCallback(async () => {
    if (!selectedFile) return;
    try {
      await invoke("clear_log_file", { path: selectedFile.path });
      setLogContent("");
      await fetchLogFiles();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, [selectedFile, fetchLogFiles]);

  const handleDeleteConfirm = useCallback(async () => {
    if (!selectedFile) return;
    try {
      await invoke("delete_log_file", { path: selectedFile.path });
      setLogContent("");
      setSelectedFile(null);
      setDeleteDialogOpen(false);
      await fetchLogFiles();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, [selectedFile, fetchLogFiles]);

  const handleOpenLogDir = useCallback(async () => {
    try {
      const dir = await invoke<string>("get_log_dir");
      await openPath(dir);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, []);

  // TLS Passthrough handlers
  const fetchTlsEntries = useCallback(async () => {
    try {
      setTlsLoading(true);
      const entries = await invoke<TlsPassthroughEntry[]>("get_tls_passthrough_list");
      setTlsEntries(entries);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    } finally {
      setTlsLoading(false);
    }
  }, []);

  const handleRemoveTlsEntry = useCallback(
    async (host: string) => {
      try {
        await invoke("remove_tls_passthrough", { host });
        await fetchTlsEntries();
      } catch (e) {
        toast.error(e instanceof Error ? e.message : String(e));
      }
    },
    [fetchTlsEntries],
  );

  const handleClearAllTls = useCallback(async () => {
    try {
      await invoke("clear_tls_passthrough");
      setTlsEntries([]);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, []);

  // Fetch log files on mount
  useEffect(() => {
    fetchLogFiles();
  }, [fetchLogFiles]);

  // Fetch content when selected file changes
  useEffect(() => {
    fetchLogContent();
  }, [fetchLogContent]);

  // Auto-refresh content every 5 seconds
  useEffect(() => {
    if (!selectedFile) return;
    const interval = setInterval(fetchLogContent, 5000);
    return () => clearInterval(interval);
  }, [selectedFile, fetchLogContent]);

  // Auto-refresh file list every 30 seconds
  useEffect(() => {
    const interval = setInterval(fetchLogFiles, 30000);
    return () => clearInterval(interval);
  }, [fetchLogFiles]);

  // Auto-scroll to bottom when content changes
  useEffect(() => {
    if (contentRef.current) {
      contentRef.current.scrollTop = contentRef.current.scrollHeight;
    }
  }, [logContent]);

  // Fetch TLS entries when tab changes
  useEffect(() => {
    if (activeTab === "tls-passthrough") {
      fetchTlsEntries();
    }
  }, [activeTab, fetchTlsEntries]);

  // TLS Passthrough 실시간 업데이트 수신
  useEffect(() => {
    const unlisten = listen<TlsPassthroughEntry[]>("tls_passthrough_updated", (event) => {
      const entries = [...event.payload];
      entries.sort((a, b) => a.host.localeCompare(b.host));
      setTlsEntries(entries);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const filteredLines = useMemo(() => {
    const lines = logContent.split("\n");
    if (!filter.trim()) return lines;
    const lowerFilter = filter.toLowerCase();
    return lines.filter((line) => line.toLowerCase().includes(lowerFilter));
  }, [logContent, filter]);

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden">
      <div className="p-6 pb-3">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-foreground">
              <Trans>Logs</Trans>
            </h1>
            <p className="text-muted-foreground">
              <Trans>View and manage application log files</Trans>
            </p>
          </div>
          <div className="flex items-center gap-2">
            {activeTab === "logs" && (
              <>
                <Button variant="outline" size="sm" onClick={handleRefresh} disabled={loading}>
                  <RefreshCw className={`w-4 h-4 mr-1 ${loading ? "animate-spin" : ""}`} />
                  <Trans>Refresh</Trans>
                </Button>
                <Button variant="outline" size="sm" onClick={handleClear} disabled={!selectedFile}>
                  <Trash2 className="w-4 h-4 mr-1" />
                  <Trans>Clear</Trans>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setDeleteDialogOpen(true)}
                  disabled={!selectedFile}
                >
                  <FileX2 className="w-4 h-4 mr-1" />
                  <Trans>Delete</Trans>
                </Button>
                <Button variant="outline" size="sm" onClick={handleOpenLogDir}>
                  <FolderOpen className="w-4 h-4 mr-1" />
                  <Trans>Open Directory</Trans>
                </Button>
              </>
            )}
            {activeTab === "tls-passthrough" && (
              <>
                <Button variant="outline" size="sm" onClick={fetchTlsEntries} disabled={tlsLoading}>
                  <RefreshCw className={`w-4 h-4 mr-1 ${tlsLoading ? "animate-spin" : ""}`} />
                  <Trans>Refresh</Trans>
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleClearAllTls}
                  disabled={tlsEntries.length === 0}
                >
                  <Trash2 className="w-4 h-4 mr-1" />
                  <Trans>Clear All</Trans>
                </Button>
              </>
            )}
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-1 mt-3 border-b">
          <button
            onClick={() => setActiveTab("logs")}
            className={`px-3 py-1.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === "logs"
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            <Trans>Log Files</Trans>
          </button>
          <button
            onClick={() => setActiveTab("tls-passthrough")}
            className={`px-3 py-1.5 text-sm font-medium border-b-2 transition-colors flex items-center gap-1.5 ${
              activeTab === "tls-passthrough"
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            <Shield className="w-3.5 h-3.5" />
            <Trans>TLS Passthrough</Trans>
            {tlsEntries.length > 0 && (
              <Badge variant="secondary" className="text-xs ml-1">
                {tlsEntries.length}
              </Badge>
            )}
          </button>
        </div>
      </div>

      {activeTab === "logs" && (
        <div className="flex-1 flex overflow-hidden px-6 pb-6 gap-4">
          {/* Left panel - file list */}
          <div className="w-[250px] shrink-0 border rounded-lg overflow-auto">
            <div className="p-3 border-b">
              <h2 className="text-sm font-semibold text-muted-foreground">
                <Trans>Log Files</Trans>
              </h2>
            </div>
            {logFiles.length === 0 ? (
              <div className="p-4 text-center text-sm text-muted-foreground">
                <Trans>No log files found</Trans>
              </div>
            ) : (
              <div className="divide-y">
                {logFiles.map((file) => (
                  <button
                    key={file.path}
                    onClick={() => setSelectedFile(file)}
                    className={`w-full text-left p-3 hover:bg-accent transition-colors ${
                      selectedFile?.path === file.path ? "bg-accent" : ""
                    }`}
                  >
                    <div className="text-sm font-medium truncate">{file.name}</div>
                    <div className="flex items-center gap-2 mt-1">
                      <Badge variant="outline" className="text-xs">
                        {formatBytes(file.size)}
                      </Badge>
                    </div>
                    <div className="text-xs text-muted-foreground mt-1">
                      {formatDate(file.modified)}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Right panel - log content */}
          <div className="flex-1 flex flex-col border rounded-lg overflow-hidden">
            <div className="p-3 border-b flex items-center gap-2">
              <Search className="w-4 h-4 text-muted-foreground" />
              <Input
                placeholder={t`Filter log lines...`}
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                className="h-8 text-sm"
              />
              {filter && (
                <Badge variant="secondary" className="text-xs shrink-0">
                  {filteredLines.length} <Trans>lines</Trans>
                </Badge>
              )}
            </div>
            <pre
              ref={contentRef}
              className="flex-1 overflow-auto p-4 text-xs font-mono bg-muted/30 m-0"
            >
              {selectedFile ? (
                filteredLines.map((line, i) => (
                  <div key={i} className={`leading-5 ${getLogLevelClass(line)}`}>
                    {line}
                  </div>
                ))
              ) : (
                <div className="text-muted-foreground text-sm text-center pt-8">
                  <Trans>Select a log file to view its contents</Trans>
                </div>
              )}
            </pre>
          </div>
        </div>
      )}

      {activeTab === "tls-passthrough" && (
        <div className="flex-1 flex flex-col overflow-hidden px-6 pb-6">
          <div className="border rounded-lg overflow-hidden flex-1 flex flex-col">
            <div className="p-3 border-b">
              <p className="text-sm text-muted-foreground">
                <Trans>
                  Domains that failed TLS handshake are automatically tunneled without MITM
                  decryption. You can remove entries to retry MITM for specific domains.
                </Trans>
              </p>
            </div>
            {tlsEntries.length === 0 ? (
              <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
                <Trans>No TLS passthrough entries</Trans>
              </div>
            ) : (
              <div className="flex-1 overflow-auto">
                <table className="w-full">
                  <thead className="sticky top-0 bg-background">
                    <tr className="border-b text-left text-xs text-muted-foreground">
                      <th className="p-3 font-medium">
                        <Trans>Domain</Trans>
                      </th>
                      <th className="p-3 font-medium w-[120px]">
                        <Trans>Failures</Trans>
                      </th>
                      <th className="p-3 font-medium w-[80px]"></th>
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {tlsEntries.map((entry) => (
                      <tr key={entry.host} className="hover:bg-accent/50 transition-colors">
                        <td className="p-3 text-sm font-mono">{entry.host}</td>
                        <td className="p-3">
                          <Badge variant="outline" className="text-xs">
                            {entry.failure_count}
                          </Badge>
                        </td>
                        <td className="p-3">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleRemoveTlsEntry(entry.host)}
                            className="h-7 w-7 p-0"
                          >
                            <X className="w-4 h-4" />
                          </Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      )}
      {/* Delete confirmation dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>
              <Trans>Delete Log File</Trans>
            </DialogTitle>
            <DialogDescription>
              <Trans>Delete "{selectedFile?.name}"? This action cannot be undone.</Trans>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose>
              <Button variant="outline">
                <Trans>Cancel</Trans>
              </Button>
            </DialogClose>
            <Button variant="destructive" onClick={handleDeleteConfirm}>
              <Trans>Delete</Trans>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
