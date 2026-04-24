import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

import { invoke } from "@tauri-apps/api/core";

import type { InterceptRule } from "../../src/entities/intercept-rule";

async function getStore() {
  const { useMapRuleStore } = await import("../../src/shared/stores/map-rule-store");
  return useMapRuleStore;
}

function createMapLocalRule(overrides: Partial<InterceptRule> = {}): InterceptRule {
  return {
    id: crypto.randomUUID(),
    name: "Test Map Local",
    enabled: true,
    pattern: "*.api.com/v1/*",
    method: null,
    action: {
      type: "map_local",
      file_path: "/tmp/mock.json",
      status_code: 200,
      headers: {},
    },
    ...overrides,
  };
}

function createMapRemoteRule(overrides: Partial<InterceptRule> = {}): InterceptRule {
  return {
    id: crypto.randomUUID(),
    name: "Test Map Remote",
    enabled: true,
    pattern: "*.api.com/*",
    method: null,
    action: {
      type: "map_remote",
      target_url: "https://staging.api.com",
      preserve_path: true,
    },
    ...overrides,
  };
}

describe("map-rule-store", () => {
  let useMapRuleStore: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    useMapRuleStore = await getStore();
    // zustand persist rehydrate가 async로 진행되는데, CI Linux에서 타이밍이
    // addRule과 겹쳐 rules를 덮어쓰는 race condition이 관측되어 명시적으로 대기한다.
    if (!useMapRuleStore.persist.hasHydrated()) {
      await new Promise<void>((resolve) => {
        const unsub = useMapRuleStore.persist.onFinishHydration(() => {
          unsub();
          resolve();
        });
      });
    }
    useMapRuleStore.setState({ rules: [] });
    (invoke as ReturnType<typeof mock>).mockClear();
  });

  describe("addRule", () => {
    test("Map Local 규칙을 추가할 수 있다", () => {
      const rule = createMapLocalRule();
      useMapRuleStore.getState().addRule(rule);

      const { rules } = useMapRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].action.type).toBe("map_local");
    });

    test("Map Remote 규칙을 추가할 수 있다", () => {
      const rule = createMapRemoteRule();
      useMapRuleStore.getState().addRule(rule);

      const { rules } = useMapRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].action.type).toBe("map_remote");
    });

    test("규칙 추가 시 syncToProxy가 호출된다", () => {
      const rule = createMapLocalRule();
      useMapRuleStore.getState().addRule(rule);

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: [rule],
      });
    });
  });

  describe("removeRule", () => {
    test("규칙을 ID로 삭제할 수 있다", () => {
      const rule = createMapLocalRule();
      useMapRuleStore.getState().addRule(rule);
      useMapRuleStore.getState().removeRule(rule.id);

      expect(useMapRuleStore.getState().rules).toHaveLength(0);
    });
  });

  describe("toggleRule", () => {
    test("활성화된 규칙을 비활성화할 수 있다", () => {
      const rule = createMapRemoteRule({ enabled: true });
      useMapRuleStore.getState().addRule(rule);
      useMapRuleStore.getState().toggleRule(rule.id);

      expect(useMapRuleStore.getState().rules[0].enabled).toBe(false);
    });
  });

  describe("setRules", () => {
    test("외부에서 규칙 목록을 덮어쓸 수 있다", () => {
      useMapRuleStore.getState().addRule(createMapLocalRule({ name: "Existing" }));

      const newRules = [
        createMapLocalRule({ name: "From MCP 1" }),
        createMapRemoteRule({ name: "From MCP 2" }),
      ];
      useMapRuleStore.getState().setRules(newRules);

      const { rules } = useMapRuleStore.getState();
      expect(rules).toHaveLength(2);
      expect(rules[0].name).toBe("From MCP 1");
      expect(rules[1].name).toBe("From MCP 2");
    });

    test("빈 배열로 설정하면 모든 규칙이 제거된다", () => {
      useMapRuleStore.getState().addRule(createMapLocalRule());
      useMapRuleStore.getState().setRules([]);

      expect(useMapRuleStore.getState().rules).toHaveLength(0);
    });

    test("setRules는 syncToProxy를 호출하지 않는다", () => {
      (invoke as ReturnType<typeof mock>).mockClear();

      useMapRuleStore.getState().setRules([createMapRemoteRule()]);

      expect(invoke).not.toHaveBeenCalled();
    });
  });
});
