import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import { Button, Badge, Input } from "@/shared/ui";
import { RefreshCw, Trash2, FolderOpen, Search } from "lucide-react";
import { toast } from "sonner";

interface LogFileInfo {
  name: string;
  path: string;
  size: number;
  modified: number;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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

export function LogsPage() {
  const { t } = useLingui();
  const [logFiles, setLogFiles] = useState<LogFileInfo[]>([]);
  const [selectedFile, setSelectedFile] = useState<LogFileInfo | null>(null);
  const [logContent, setLogContent] = useState("");
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(false);
  const contentRef = useRef<HTMLPreElement>(null);

  const fetchLogFiles = useCallback(async () => {
    try {
      const files = await invoke<LogFileInfo[]>("get_log_files");
      setLogFiles(files);
      // Auto-select first file if none selected
      if (!selectedFile && files.length > 0) {
        setSelectedFile(files[0]);
      }
      // Update selected file info if it still exists
      if (selectedFile) {
        const updated = files.find((f) => f.path === selectedFile.path);
        if (updated) {
          setSelectedFile(updated);
        }
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, [selectedFile]);

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

  const handleOpenLogDir = useCallback(async () => {
    try {
      const dir = await invoke<string>("get_log_dir");
      await openPath(dir);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : String(e));
    }
  }, []);

  // Fetch log files on mount
  useEffect(() => {
    fetchLogFiles();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

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
            <Button variant="outline" size="sm" onClick={handleRefresh} disabled={loading}>
              <RefreshCw className={`w-4 h-4 mr-1 ${loading ? "animate-spin" : ""}`} />
              <Trans>Refresh</Trans>
            </Button>
            <Button variant="outline" size="sm" onClick={handleClear} disabled={!selectedFile}>
              <Trash2 className="w-4 h-4 mr-1" />
              <Trans>Clear</Trans>
            </Button>
            <Button variant="outline" size="sm" onClick={handleOpenLogDir}>
              <FolderOpen className="w-4 h-4 mr-1" />
              <Trans>Open Directory</Trans>
            </Button>
          </div>
        </div>
      </div>

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
                      {formatFileSize(file.size)}
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
    </div>
  );
}
