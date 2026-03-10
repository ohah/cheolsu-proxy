import { useState, useEffect } from "react";
import { readFile, BaseDirectory } from "@tauri-apps/plugin-fs";

interface UseBodyFileResult {
  body: Uint8Array | null;
  loading: boolean;
  error: string | null;
}

/**
 * 파일에서 body 데이터를 읽어오는 훅
 * @param filePath - 읽어올 파일 경로
 * @param enabled - 파일 읽기를 활성화할지 여부
 */
export function useBodyFile(
  filePath: string | undefined,
  enabled: boolean = true,
): UseBodyFileResult {
  const [body, setBody] = useState<Uint8Array | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled || !filePath) {
      setBody(null);
      setError(null);
      return;
    }

    const loadBody = async () => {
      setLoading(true);
      setError(null);

      try {
        // com.cheolsu-proxy/ 접두사가 있으면 제거하고 AppCache 기준으로 읽기
        const appCachePath = filePath.startsWith("com.cheolsu-proxy/")
          ? filePath.slice("com.cheolsu-proxy/".length)
          : filePath;
        const rawData = await readFile(appCachePath, { baseDir: BaseDirectory.AppCache });
        setBody(rawData);
      } catch (err) {
        setError(err instanceof Error ? err.message : "파일 읽기 실패");
        setBody(null);
      } finally {
        setLoading(false);
      }
    };

    loadBody();
  }, [filePath, enabled]);

  return { body, loading, error };
}
