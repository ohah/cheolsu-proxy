import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

import { invoke } from "@tauri-apps/api/core";

import type { InterceptRule } from "../../src/entities/intercept-rule";

function createBlockRule(id: string): InterceptRule {
  return {
    id,
    name: `Rule ${id}`,
    enabled: true,
    pattern: "*.example.com/*",
    method: null,
    action: { type: "block", status_code: 403, body: "" },
  };
}

function createMapLocalRule(id: string): InterceptRule {
  return {
    id,
    name: `Map ${id}`,
    enabled: true,
    pattern: "*.api.com/*",
    method: null,
    action: { type: "map_local", file_path: "/tmp/mock.json", status_code: 200, headers: {} },
  };
}

import {
  registerRuleStore,
  updateDaemonRules,
  syncAllRulesToProxy,
  waitForDaemonRules,
  _resetForTest,
} from "../../src/shared/stores/sync-rules";

describe("sync-rules", () => {
  beforeEach(() => {
    _resetForTest();
    (invoke as ReturnType<typeof mock>).mockClear();
  });

  describe("syncAllRulesToProxy", () => {
    test("앱 store 규칙만 있을 때 정상 전송", async () => {
      const appRules = [createBlockRule("app-1"), createBlockRule("app-2")];
      registerRuleStore(() => ({ rules: appRules }));

      await syncAllRulesToProxy();

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: appRules,
      });
    });

    test("외부 규칙(MCP)이 있을 때 앱 규칙과 병합하여 전송", async () => {
      const appRule = createBlockRule("app-1");
      const mcpRule = createBlockRule("mcp_0");

      registerRuleStore(() => ({ rules: [appRule] }));
      updateDaemonRules([appRule, mcpRule]);

      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(2);
      expect(sentRules.some((r: InterceptRule) => r.id === "app-1")).toBe(true);
      expect(sentRules.some((r: InterceptRule) => r.id === "mcp_0")).toBe(true);
    });

    test("앱 store에 없는 외부 규칙만 보존", async () => {
      const appRule = createBlockRule("app-1");
      const mcpRule1 = createBlockRule("mcp_0");
      const mcpRule2 = createBlockRule("mcp_1");

      registerRuleStore(() => ({ rules: [appRule] }));
      // 데몬에는 앱 규칙 + MCP 규칙 2개
      updateDaemonRules([appRule, mcpRule1, mcpRule2]);

      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(3);
      // 앱 규칙이 먼저, 외부 규칙이 뒤에
      expect(sentRules[0].id).toBe("app-1");
      expect(sentRules.some((r: InterceptRule) => r.id === "mcp_0")).toBe(true);
      expect(sentRules.some((r: InterceptRule) => r.id === "mcp_1")).toBe(true);
    });

    test("daemonRules가 비어있으면 앱 규칙만 전송", async () => {
      const appRules = [createBlockRule("app-1")];
      registerRuleStore(() => ({ rules: appRules }));

      await syncAllRulesToProxy();

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: appRules,
      });
    });

    test("여러 store의 규칙을 합쳐서 전송", async () => {
      const interceptRule = createBlockRule("intercept-1");
      const mapRule = createMapLocalRule("map-1");
      const mcpRule = createBlockRule("mcp_0");

      registerRuleStore(() => ({ rules: [interceptRule] }));
      registerRuleStore(() => ({ rules: [mapRule] }));
      updateDaemonRules([interceptRule, mapRule, mcpRule]);

      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(3);
      expect(sentRules.some((r: InterceptRule) => r.id === "intercept-1")).toBe(true);
      expect(sentRules.some((r: InterceptRule) => r.id === "map-1")).toBe(true);
      expect(sentRules.some((r: InterceptRule) => r.id === "mcp_0")).toBe(true);
    });

    test("앱에서 규칙 삭제 후에도 외부 규칙 보존", async () => {
      const mcpRule = createBlockRule("mcp_0");

      // 앱 규칙이 전부 삭제된 상태
      registerRuleStore(() => ({ rules: [] }));
      updateDaemonRules([mcpRule]);

      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(1);
      expect(sentRules[0].id).toBe("mcp_0");
    });
  });

  describe("updateDaemonRules", () => {
    test("데몬 규칙 업데이트", async () => {
      const mcpRule = createBlockRule("mcp_0");

      registerRuleStore(() => ({ rules: [] }));
      updateDaemonRules([mcpRule]);
      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(1);
      expect(sentRules[0].id).toBe("mcp_0");
    });

    test("데몬 규칙 업데이트 시 이전 규칙 대체", async () => {
      registerRuleStore(() => ({ rules: [] }));

      // 첫 번째 업데이트
      updateDaemonRules([createBlockRule("mcp_0"), createBlockRule("mcp_1")]);
      // 두 번째 업데이트 (mcp_1 제거됨)
      updateDaemonRules([createBlockRule("mcp_0")]);

      await syncAllRulesToProxy();

      const calledWith = (invoke as ReturnType<typeof mock>).mock.calls[0];
      const sentRules = calledWith[1].rules as InterceptRule[];
      expect(sentRules).toHaveLength(1);
      expect(sentRules[0].id).toBe("mcp_0");
    });
  });

  describe("waitForDaemonRules", () => {
    test("updateDaemonRules 호출 후 즉시 resolve", async () => {
      updateDaemonRules([]);
      // 이미 resolve되었으므로 즉시 완료
      await waitForDaemonRules();
    });

    test("타임아웃 내에 resolve", async () => {
      // 100ms 후 updateDaemonRules 호출
      setTimeout(() => updateDaemonRules([]), 100);

      const start = Date.now();
      await waitForDaemonRules();
      const elapsed = Date.now() - start;

      // 2초 타임아웃 전에 완료되어야 함
      expect(elapsed).toBeLessThan(2000);
    });
  });
});
