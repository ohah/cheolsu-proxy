import type { AdvancedRepeatResult, ReplayResponse, SequenceReplayResult } from "@/shared/api/proxy";

const handlers: Record<string, (...args: any[]) => any> = {
  start_proxy_v2: async () => ({ status: true, message: "Mock proxy started" }),
  stop_proxy_v2: async () => {},
  proxy_v2_status: async () => true,
  clean_old_proxy_cache: async () => "Mock: cache cleaned",
  update_intercept_rules_v2: async () => {},

  replay_request: async (_args: { params: any }): Promise<ReplayResponse> => {
    await new Promise((r) => setTimeout(r, 200 + Math.random() * 300));
    return {
      status: 200,
      headers: { "content-type": "application/json", "x-mock": "true" },
      body: JSON.stringify({
        mock: true,
        message: "Mock replay response",
        timestamp: new Date().toISOString(),
      }),
      body_size: 80,
      elapsed_ms: Math.round(200 + Math.random() * 300),
    };
  },

  replay_sequence: async (args: { requests: any[] }): Promise<SequenceReplayResult[]> => {
    const results: SequenceReplayResult[] = [];
    for (let i = 0; i < args.requests.length; i++) {
      await new Promise((r) => setTimeout(r, 100));
      results.push({
        index: i,
        url: args.requests[i].url,
        method: args.requests[i].method,
        response: {
          status: 200,
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ mock: true, index: i }),
          body_size: 30,
          elapsed_ms: Math.round(100 + Math.random() * 200),
        },
      });
    }
    return results;
  },

  advanced_repeat: async (args: { params: any }): Promise<AdvancedRepeatResult> => {
    const { iterations = 10 } = args.params;
    await new Promise((r) => setTimeout(r, iterations * 50));
    const avgTime = 150 + Math.random() * 100;
    return {
      total: iterations,
      success_count: iterations - Math.floor(Math.random() * 2),
      failure_count: Math.floor(Math.random() * 2),
      min_time_ms: Math.round(avgTime * 0.5),
      max_time_ms: Math.round(avgTime * 2),
      avg_time_ms: Math.round(avgTime * 100) / 100,
      total_time_ms: Math.round(iterations * avgTime * 0.3),
      requests_per_second: Math.round((1000 / avgTime) * iterations * 0.3 * 100) / 100,
      status_codes: { 200: iterations - Math.floor(Math.random() * 2), 500: Math.floor(Math.random() * 2) },
    };
  },
};

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[cmd];
  if (handler) {
    return handler(args) as Promise<T>;
  }
  return undefined as T;
}
