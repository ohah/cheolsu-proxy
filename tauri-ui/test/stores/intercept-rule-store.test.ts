import { describe, test, expect, beforeEach, mock } from "bun:test";
import "../../test/mocks/tauri";

// invoke mock 참조
import { invoke } from "@tauri-apps/api/core";

import type { InterceptRule } from "../../src/entities/intercept-rule";

// 각 테스트 전에 스토어를 초기화하기 위해 동적으로 가져옴
async function getStore() {
  const { useInterceptRuleStore } = await import("../../src/shared/stores/intercept-rule-store");
  return useInterceptRuleStore;
}

function createBlockRule(overrides: Partial<InterceptRule> = {}): InterceptRule {
  return {
    id: crypto.randomUUID(),
    name: "Test Block Rule",
    enabled: true,
    pattern: "*.example.com/*",
    method: null,
    action: { type: "block", status_code: 403, body: "" },
    ...overrides,
  };
}

function createModifyRequestRule(overrides: Partial<InterceptRule> = {}): InterceptRule {
  return {
    id: crypto.randomUUID(),
    name: "Test ModifyRequest Rule",
    enabled: true,
    pattern: "*.api.com/*",
    method: "GET",
    action: {
      type: "modify_request",
      add_headers: { "X-Debug": "true" },
      remove_headers: [],
      set_body: null,
    },
    ...overrides,
  };
}

function createModifyResponseRule(overrides: Partial<InterceptRule> = {}): InterceptRule {
  return {
    id: crypto.randomUUID(),
    name: "Test ModifyResponse Rule",
    enabled: true,
    pattern: "*.api.com/v2/*",
    method: null,
    action: {
      type: "modify_response",
      set_status: 200,
      add_headers: {},
      remove_headers: ["X-Powered-By"],
      set_body: '{"mocked":true}',
    },
    ...overrides,
  };
}

describe("intercept-rule-store", () => {
  let useInterceptRuleStore: Awaited<ReturnType<typeof getStore>>;

  beforeEach(async () => {
    useInterceptRuleStore = await getStore();
    // 스토어 초기화
    useInterceptRuleStore.setState({ rules: [] });
    (invoke as ReturnType<typeof mock>).mockClear();
  });

  describe("addRule", () => {
    test("Block 규칙을 추가하면 rules에 포함된다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0]).toEqual(rule);
    });

    test("ModifyRequest 규칙을 추가할 수 있다", () => {
      const rule = createModifyRequestRule();
      useInterceptRuleStore.getState().addRule(rule);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].action.type).toBe("modify_request");
    });

    test("ModifyResponse 규칙을 추가할 수 있다", () => {
      const rule = createModifyResponseRule();
      useInterceptRuleStore.getState().addRule(rule);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].action.type).toBe("modify_response");
      if (rules[0].action.type === "modify_response") {
        expect(rules[0].action.set_status).toBe(200);
        expect(rules[0].action.set_body).toBe('{"mocked":true}');
      }
    });

    test("여러 규칙을 순서대로 추가할 수 있다", () => {
      const rule1 = createBlockRule({ name: "Rule 1" });
      const rule2 = createModifyRequestRule({ name: "Rule 2" });
      const rule3 = createModifyResponseRule({ name: "Rule 3" });

      const store = useInterceptRuleStore.getState();
      store.addRule(rule1);
      store.addRule(rule2);
      store.addRule(rule3);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(3);
      expect(rules[0].name).toBe("Rule 1");
      expect(rules[1].name).toBe("Rule 2");
      expect(rules[2].name).toBe("Rule 3");
    });

    test("규칙 추가 시 syncToProxy가 호출된다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: [rule],
      });
    });
  });

  describe("updateRule", () => {
    test("기존 규칙의 패턴을 수정할 수 있다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);

      const updated = { ...rule, pattern: "*.newdomain.com/*" };
      useInterceptRuleStore.getState().updateRule(updated);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules[0].pattern).toBe("*.newdomain.com/*");
    });

    test("규칙의 액션 타입을 변경할 수 있다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);

      const updated: InterceptRule = {
        ...rule,
        action: {
          type: "modify_response",
          set_status: 200,
          add_headers: {},
          remove_headers: [],
          set_body: '{"ok":true}',
        },
      };
      useInterceptRuleStore.getState().updateRule(updated);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules[0].action.type).toBe("modify_response");
    });

    test("존재하지 않는 ID로 업데이트하면 변경이 없다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);

      const fakeRule = createBlockRule({ id: "nonexistent", name: "Fake" });
      useInterceptRuleStore.getState().updateRule(fakeRule);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].id).toBe(rule.id);
    });
  });

  describe("removeRule", () => {
    test("규칙을 ID로 삭제할 수 있다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);
      useInterceptRuleStore.getState().removeRule(rule.id);

      expect(useInterceptRuleStore.getState().rules).toHaveLength(0);
    });

    test("여러 규칙 중 특정 규칙만 삭제한다", () => {
      const rule1 = createBlockRule({ name: "Keep" });
      const rule2 = createModifyRequestRule({ name: "Delete" });
      useInterceptRuleStore.getState().addRule(rule1);
      useInterceptRuleStore.getState().addRule(rule2);

      useInterceptRuleStore.getState().removeRule(rule2.id);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules).toHaveLength(1);
      expect(rules[0].name).toBe("Keep");
    });

    test("존재하지 않는 ID로 삭제해도 에러가 발생하지 않는다", () => {
      const rule = createBlockRule();
      useInterceptRuleStore.getState().addRule(rule);
      useInterceptRuleStore.getState().removeRule("nonexistent");

      expect(useInterceptRuleStore.getState().rules).toHaveLength(1);
    });
  });

  describe("toggleRule", () => {
    test("활성화된 규칙을 비활성화할 수 있다", () => {
      const rule = createBlockRule({ enabled: true });
      useInterceptRuleStore.getState().addRule(rule);
      useInterceptRuleStore.getState().toggleRule(rule.id);

      expect(useInterceptRuleStore.getState().rules[0].enabled).toBe(false);
    });

    test("비활성화된 규칙을 활성화할 수 있다", () => {
      const rule = createBlockRule({ enabled: false });
      useInterceptRuleStore.getState().addRule(rule);
      useInterceptRuleStore.getState().toggleRule(rule.id);

      expect(useInterceptRuleStore.getState().rules[0].enabled).toBe(true);
    });

    test("토글해도 다른 규칙에 영향을 주지 않는다", () => {
      const rule1 = createBlockRule({ enabled: true });
      const rule2 = createModifyRequestRule({ enabled: true });
      useInterceptRuleStore.getState().addRule(rule1);
      useInterceptRuleStore.getState().addRule(rule2);

      useInterceptRuleStore.getState().toggleRule(rule1.id);

      const { rules } = useInterceptRuleStore.getState();
      expect(rules[0].enabled).toBe(false);
      expect(rules[1].enabled).toBe(true);
    });
  });

  describe("clearRules", () => {
    test("모든 규칙을 삭제한다", () => {
      useInterceptRuleStore.getState().addRule(createBlockRule());
      useInterceptRuleStore.getState().addRule(createModifyRequestRule());
      useInterceptRuleStore.getState().addRule(createModifyResponseRule());

      useInterceptRuleStore.getState().clearRules();

      expect(useInterceptRuleStore.getState().rules).toHaveLength(0);
    });

    test("빈 상태에서 clearRules 호출해도 에러가 없다", () => {
      useInterceptRuleStore.getState().clearRules();
      expect(useInterceptRuleStore.getState().rules).toHaveLength(0);
    });

    test("clearRules 시 빈 배열로 syncToProxy가 호출된다", () => {
      useInterceptRuleStore.getState().addRule(createBlockRule());
      (invoke as ReturnType<typeof mock>).mockClear();

      useInterceptRuleStore.getState().clearRules();

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: [],
      });
    });
  });

  describe("syncToProxy", () => {
    test("현재 규칙 전체를 invoke로 전달한다", () => {
      const rule1 = createBlockRule();
      const rule2 = createModifyRequestRule();
      useInterceptRuleStore.getState().addRule(rule1);
      useInterceptRuleStore.getState().addRule(rule2);
      (invoke as ReturnType<typeof mock>).mockClear();

      useInterceptRuleStore.getState().syncToProxy();

      expect(invoke).toHaveBeenCalledWith("update_intercept_rules_v2", {
        rules: [rule1, rule2],
      });
    });

    test("invoke 실패 시 에러를 콘솔에 출력하고 throw하지 않는다", async () => {
      (invoke as ReturnType<typeof mock>).mockRejectedValueOnce(new Error("Connection failed"));

      // 에러가 throw되지 않아야 한다
      await useInterceptRuleStore.getState().syncToProxy();
    });
  });
});
