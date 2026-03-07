import { useCallback, useRef, useEffect } from "react";

interface WorkerTask {
  id: string;
  resolve: (value: string) => void;
  reject: (error: Error) => void;
}

interface WorkerMessage {
  id: string;
  data: Uint8Array | number[];
  dataType: string;
}

interface WorkerResponse {
  id: string;
  success: boolean;
  dataUrl?: string;
  error?: string;
}

/**
 * Base64 변환을 위한 Web Worker 관리 Hook
 * 메인 스레드를 블로킹하지 않고 이미지 데이터를 Base64로 변환
 */
export const useBase64Worker = () => {
  const workerRef = useRef<Worker | null>(null);
  const tasksRef = useRef<Map<string, WorkerTask>>(new Map());
  const taskIdRef = useRef(0);

  useEffect(() => {
    // Worker 생성
    try {
      workerRef.current = new Worker(new URL("../workers/base64-worker.ts", import.meta.url));

      workerRef.current.onmessage = (e: MessageEvent<WorkerResponse>) => {
        const { id, success, dataUrl, error } = e.data;
        const task = tasksRef.current.get(id);

        if (task) {
          tasksRef.current.delete(id);
          if (success && dataUrl) {
            task.resolve(dataUrl);
          } else {
            task.reject(new Error(error || "Unknown error"));
          }
        }
      };

      workerRef.current.onerror = () => {
        // 모든 대기 중인 작업을 실패로 처리
        tasksRef.current.forEach((task) => {
          task.reject(new Error("Worker error"));
        });
        tasksRef.current.clear();
      };
    } catch {
      // Worker 생성 실패 시 조용히 처리
    }

    return () => {
      if (workerRef.current) {
        // 대기 중인 모든 작업을 실패로 처리
        tasksRef.current.forEach((task) => {
          task.reject(new Error("Worker terminated"));
        });
        tasksRef.current.clear();

        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, []);

  /**
   * 이미지 데이터를 Base64 Data URL로 변환
   */
  const encodeToBase64 = useCallback(
    (data: Uint8Array | number[], dataType: string): Promise<string> => {
      return new Promise((resolve, reject) => {
        if (!workerRef.current) {
          reject(new Error("Worker not initialized"));
          return;
        }

        taskIdRef.current += 1;
        const id = `task_${taskIdRef.current}`;
        tasksRef.current.set(id, { id, resolve, reject });

        const message: WorkerMessage = {
          id,
          data,
          dataType,
        };

        try {
          workerRef.current.postMessage(message);
        } catch {
          tasksRef.current.delete(id);
          reject(new Error("Failed to send message to worker"));
        }
      });
    },
    [],
  );

  /**
   * Worker가 사용 가능한지 확인
   */
  const isWorkerAvailable = useCallback(() => {
    return workerRef.current !== null;
  }, []);

  return {
    encodeToBase64,
    isWorkerAvailable,
  };
};
