import { describe, test, expect } from "bun:test";
import type {
  InterceptRule,
  InterceptAction,
  BlockAction,
  ModifyRequestAction,
  ModifyResponseAction,
  InterceptActionType,
} from "../../src/entities/intercept-rule";

describe("intercept-rule types", () => {
  describe("BlockAction", () => {
    test("Block 액션 구조가 올바르다", () => {
      const action: BlockAction = {
        type: "block",
        status_code: 403,
        body: "Forbidden",
      };

      expect(action.type).toBe("block");
      expect(action.status_code).toBe(403);
      expect(action.body).toBe("Forbidden");
    });

    test("Block 액션에 빈 body를 허용한다", () => {
      const action: BlockAction = {
        type: "block",
        status_code: 404,
        body: "",
      };

      expect(action.body).toBe("");
    });
  });

  describe("ModifyRequestAction", () => {
    test("ModifyRequest 액션에 헤더 추가/제거를 설정할 수 있다", () => {
      const action: ModifyRequestAction = {
        type: "modify_request",
        add_headers: { "X-Custom": "value", Authorization: "Bearer token" },
        remove_headers: ["Cookie"],
        set_body: null,
      };

      expect(action.type).toBe("modify_request");
      expect(Object.keys(action.add_headers)).toHaveLength(2);
      expect(action.remove_headers).toContain("Cookie");
      expect(action.set_body).toBeNull();
    });

    test("ModifyRequest 액션에 body를 설정할 수 있다", () => {
      const action: ModifyRequestAction = {
        type: "modify_request",
        add_headers: {},
        remove_headers: [],
        set_body: '{"overridden": true}',
      };

      expect(action.set_body).toBe('{"overridden": true}');
    });
  });

  describe("ModifyResponseAction", () => {
    test("ModifyResponse 액션에 상태코드와 body를 설정할 수 있다", () => {
      const action: ModifyResponseAction = {
        type: "modify_response",
        set_status: 200,
        add_headers: { "X-Mock": "true" },
        remove_headers: ["X-Powered-By"],
        set_body: '{"mocked": true}',
      };

      expect(action.type).toBe("modify_response");
      expect(action.set_status).toBe(200);
      expect(action.set_body).toBe('{"mocked": true}');
    });

    test("ModifyResponse 액션에 set_status를 null로 설정할 수 있다", () => {
      const action: ModifyResponseAction = {
        type: "modify_response",
        set_status: null,
        add_headers: {},
        remove_headers: [],
        set_body: null,
      };

      expect(action.set_status).toBeNull();
    });
  });

  describe("InterceptRule", () => {
    test("Block 규칙 구조가 올바르다", () => {
      const rule: InterceptRule = {
        id: "rule-1",
        name: "Block Ads",
        enabled: true,
        pattern: "*ads.example.com*",
        method: null,
        action: { type: "block", status_code: 403, body: "" },
      };

      expect(rule.id).toBe("rule-1");
      expect(rule.enabled).toBe(true);
      expect(rule.method).toBeNull();
      expect(rule.action.type).toBe("block");
    });

    test("메서드 필터가 있는 규칙을 생성할 수 있다", () => {
      const rule: InterceptRule = {
        id: "rule-2",
        name: "Modify GET only",
        enabled: true,
        pattern: "*.api.com/*",
        method: "GET",
        action: {
          type: "modify_request",
          add_headers: { "X-Debug": "true" },
          remove_headers: [],
          set_body: null,
        },
      };

      expect(rule.method).toBe("GET");
    });

    test("비활성화된 규칙을 생성할 수 있다", () => {
      const rule: InterceptRule = {
        id: "rule-3",
        name: "Disabled Rule",
        enabled: false,
        pattern: "*.test.com/*",
        method: null,
        action: { type: "block", status_code: 500, body: "Error" },
      };

      expect(rule.enabled).toBe(false);
    });
  });

  describe("InterceptAction union type", () => {
    test("action type으로 액션 종류를 구분할 수 있다 (discriminated union)", () => {
      const actions: InterceptAction[] = [
        { type: "block", status_code: 403, body: "" },
        {
          type: "modify_request",
          add_headers: {},
          remove_headers: [],
          set_body: null,
        },
        {
          type: "modify_response",
          set_status: null,
          add_headers: {},
          remove_headers: [],
          set_body: null,
        },
      ];

      const types = actions.map((a) => a.type);
      expect(types).toEqual(["block", "modify_request", "modify_response"]);
    });

    test("InterceptActionType이 올바른 리터럴 타입이다", () => {
      const actionTypes: InterceptActionType[] = ["block", "modify_request", "modify_response"];

      expect(actionTypes).toHaveLength(3);
      expect(actionTypes).toContain("block");
      expect(actionTypes).toContain("modify_request");
      expect(actionTypes).toContain("modify_response");
    });
  });

  describe("와일드카드 패턴 형식", () => {
    test("다양한 와일드카드 패턴 형식을 규칙에 저장할 수 있다", () => {
      const patterns = [
        "*",
        "*.example.com",
        "*.example.com/api/*",
        "http?://example.com/*",
        "*ads*",
        "*.cdn.example.com/images/*.jpg",
      ];

      const rules: InterceptRule[] = patterns.map((pattern, i) => ({
        id: `rule-${i}`,
        name: `Pattern ${i}`,
        enabled: true,
        pattern,
        method: null,
        action: { type: "block" as const, status_code: 403, body: "" },
      }));

      expect(rules).toHaveLength(6);
      expect(rules.map((r) => r.pattern)).toEqual(patterns);
    });
  });
});
