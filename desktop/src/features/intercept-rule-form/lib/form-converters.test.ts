import { describe, test, expect } from "bun:test";

import type { InterceptRule } from "@/entities/intercept-rule";
import type { InterceptRuleInitialValues } from "@/shared/stores";

import { formValuesToAction, ruleToFormValues, initialValuesToFormValues } from "./form-converters";

describe("formValuesToAction", () => {
  describe("block 액션", () => {
    test("status_code와 body를 올바르게 변환한다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: { type: "block", status_code: "404", body: "Not Found" },
      });

      expect(result).toEqual({
        type: "block",
        status_code: 404,
        body: "Not Found",
      });
    });

    test("잘못된 status_code는 403으로 기본 설정된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: { type: "block", status_code: "abc", body: "" },
      });

      expect(result).toEqual({
        type: "block",
        status_code: 403,
        body: "",
      });
    });

    test("빈 status_code는 403으로 기본 설정된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: { type: "block", status_code: "", body: "" },
      });

      expect(result.type).toBe("block");
      expect((result as { status_code: number }).status_code).toBe(403);
    });
  });

  describe("modify_request 액션", () => {
    test("헤더와 body를 올바르게 변환한다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_request",
          headers: [
            { key: "X-Custom", value: "val1" },
            { key: "Authorization", value: "Bearer token" },
          ],
          remove_headers: ["Cookie", "X-Old"],
          body: '{"override": true}',
        },
      });

      expect(result).toEqual({
        type: "modify_request",
        add_headers: { "X-Custom": "val1", Authorization: "Bearer token" },
        remove_headers: ["Cookie", "X-Old"],
        set_body: '{"override": true}',
      });
    });

    test("빈 키를 가진 헤더는 무시된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_request",
          headers: [
            { key: "", value: "ignored" },
            { key: "  ", value: "also ignored" },
            { key: "X-Valid", value: "kept" },
          ],
          remove_headers: [],
          body: "",
        },
      });

      expect(result.type).toBe("modify_request");
      const action = result as { add_headers: Record<string, string> };
      expect(action.add_headers).toEqual({ "X-Valid": "kept" });
    });

    test("빈 body는 null로 변환된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_request",
          headers: [],
          remove_headers: [],
          body: "",
        },
      });

      expect((result as { set_body: string | null }).set_body).toBeNull();
    });

    test("공백만 있는 body는 null로 변환된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_request",
          headers: [],
          remove_headers: [],
          body: "   ",
        },
      });

      expect((result as { set_body: string | null }).set_body).toBeNull();
    });

    test("공백만 있는 remove_headers는 필터링된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_request",
          headers: [],
          remove_headers: ["Cookie", "", "  ", "X-Old"],
          body: "",
        },
      });

      expect((result as { remove_headers: string[] }).remove_headers).toEqual(["Cookie", "X-Old"]);
    });
  });

  describe("modify_response 액션", () => {
    test("상태코드, 헤더, body를 올바르게 변환한다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_response",
          response_status: "200",
          headers: [{ key: "X-Mock", value: "true" }],
          remove_headers: ["X-Powered-By"],
          body: '{"mocked": true}',
        },
      });

      expect(result).toEqual({
        type: "modify_response",
        set_status: 200,
        add_headers: { "X-Mock": "true" },
        remove_headers: ["X-Powered-By"],
        set_body: '{"mocked": true}',
      });
    });

    test("빈 response_status는 null로 변환된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_response",
          response_status: "",
          headers: [],
          remove_headers: [],
          body: "",
        },
      });

      expect((result as { set_status: number | null }).set_status).toBeNull();
    });

    test("잘못된 response_status는 null로 변환된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "modify_response",
          response_status: "abc",
          headers: [],
          remove_headers: [],
          body: "",
        },
      });

      expect((result as { set_status: number | null }).set_status).toBeNull();
    });
  });

  describe("rewrite 액션", () => {
    test("rewrite 필드를 올바르게 변환한다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "rewrite",
          rewrite_target: "request_header",
          match_pattern: "old-value",
          replace_with: "new-value",
        },
      });

      expect(result).toEqual({
        type: "rewrite",
        target: "request_header",
        match_pattern: "old-value",
        replace_with: "new-value",
      });
    });

    test("모든 rewrite target 타입을 지원한다", () => {
      const targets = [
        "request_header",
        "response_header",
        "request_body",
        "response_body",
      ] as const;

      for (const target of targets) {
        const result = formValuesToAction({
          name: "test",
          pattern: "*",
          method: "*",
          action: {
            type: "rewrite",
            rewrite_target: target,
            match_pattern: "pattern",
            replace_with: "replacement",
          },
        });

        expect(result.type).toBe("rewrite");
        expect((result as { target: string }).target).toBe(target);
      }
    });
  });

  describe("throttle 액션", () => {
    test("latency와 속도를 올바르게 변환한다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "throttle",
          latency_ms: "500",
          download_speed: "100",
          upload_speed: "50",
        },
      });

      expect(result).toEqual({
        type: "throttle",
        latency_ms: 500,
        download_rate: 100 * 1024,
        upload_rate: 50 * 1024,
      });
    });

    test("빈 속도 값은 null로 변환된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "throttle",
          latency_ms: "100",
          download_speed: "",
          upload_speed: "",
        },
      });

      expect((result as { download_rate: number | null }).download_rate).toBeNull();
      expect((result as { upload_rate: number | null }).upload_rate).toBeNull();
    });

    test("잘못된 latency_ms는 0으로 기본 설정된다", () => {
      const result = formValuesToAction({
        name: "test",
        pattern: "*",
        method: "*",
        action: {
          type: "throttle",
          latency_ms: "abc",
          download_speed: "",
          upload_speed: "",
        },
      });

      expect((result as { latency_ms: number }).latency_ms).toBe(0);
    });
  });
});

describe("ruleToFormValues", () => {
  describe("block 규칙", () => {
    test("block 규칙을 폼 값으로 변환한다", () => {
      const rule: InterceptRule = {
        id: "rule-1",
        name: "Block Ads",
        enabled: true,
        pattern: "*ads.example.com*",
        method: "GET",
        action: { type: "block", status_code: 403, body: "Blocked" },
      };

      const result = ruleToFormValues(rule);

      expect(result).toEqual({
        name: "Block Ads",
        pattern: "*ads.example.com*",
        method: "GET",
        action: { type: "block", status_code: "403", body: "Blocked" },
      });
    });

    test("method가 null이면 '*'로 변환된다", () => {
      const rule: InterceptRule = {
        id: "rule-1",
        name: "test",
        enabled: true,
        pattern: "*",
        method: null,
        action: { type: "block", status_code: 403, body: "" },
      };

      const result = ruleToFormValues(rule);
      expect(result.method).toBe("*");
    });
  });

  describe("modify_request 규칙", () => {
    test("modify_request 규칙을 폼 값으로 변환한다", () => {
      const rule: InterceptRule = {
        id: "rule-2",
        name: "Add Headers",
        enabled: true,
        pattern: "*.api.com/*",
        method: "POST",
        action: {
          type: "modify_request",
          add_headers: { "X-Custom": "val", Authorization: "Bearer tok" },
          remove_headers: ["Cookie"],
          set_body: '{"data": 1}',
        },
      };

      const result = ruleToFormValues(rule);

      expect(result.name).toBe("Add Headers");
      expect(result.action.type).toBe("modify_request");

      const action = result.action as {
        type: "modify_request";
        headers: { key: string; value: string }[];
        remove_headers: string[];
        body: string;
      };
      expect(action.headers).toEqual([
        { key: "X-Custom", value: "val" },
        { key: "Authorization", value: "Bearer tok" },
      ]);
      expect(action.remove_headers).toEqual(["Cookie"]);
      expect(action.body).toBe('{"data": 1}');
    });

    test("set_body가 null이면 빈 문자열로 변환된다", () => {
      const rule: InterceptRule = {
        id: "rule-2",
        name: "test",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "modify_request",
          add_headers: {},
          remove_headers: [],
          set_body: null,
        },
      };

      const result = ruleToFormValues(rule);
      const action = result.action as { body: string };
      expect(action.body).toBe("");
    });
  });

  describe("modify_response 규칙", () => {
    test("modify_response 규칙을 폼 값으로 변환한다", () => {
      const rule: InterceptRule = {
        id: "rule-3",
        name: "Mock Response",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "modify_response",
          set_status: 200,
          add_headers: { "X-Mock": "true" },
          remove_headers: ["X-Powered-By"],
          set_body: '{"mocked": true}',
        },
      };

      const result = ruleToFormValues(rule);

      const action = result.action as {
        type: "modify_response";
        response_status: string;
        headers: { key: string; value: string }[];
        remove_headers: string[];
        body: string;
      };
      expect(action.type).toBe("modify_response");
      expect(action.response_status).toBe("200");
      expect(action.headers).toEqual([{ key: "X-Mock", value: "true" }]);
      expect(action.remove_headers).toEqual(["X-Powered-By"]);
      expect(action.body).toBe('{"mocked": true}');
    });

    test("set_status가 null이면 빈 문자열로 변환된다", () => {
      const rule: InterceptRule = {
        id: "rule-3",
        name: "test",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "modify_response",
          set_status: null,
          add_headers: {},
          remove_headers: [],
          set_body: null,
        },
      };

      const result = ruleToFormValues(rule);
      const action = result.action as { response_status: string };
      expect(action.response_status).toBe("");
    });
  });

  describe("rewrite 규칙", () => {
    test("rewrite 규칙을 폼 값으로 변환한다", () => {
      const rule: InterceptRule = {
        id: "rule-4",
        name: "Rewrite Header",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "rewrite",
          target: "request_header",
          match_pattern: "old",
          replace_with: "new",
        },
      };

      const result = ruleToFormValues(rule);

      expect(result.action).toEqual({
        type: "rewrite",
        rewrite_target: "request_header",
        match_pattern: "old",
        replace_with: "new",
      });
    });
  });

  describe("throttle 규칙", () => {
    test("throttle 규칙을 폼 값으로 변환한다", () => {
      const rule: InterceptRule = {
        id: "rule-5",
        name: "Slow Connection",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "throttle",
          latency_ms: 500,
          download_rate: 102400,
          upload_rate: 51200,
        },
      };

      const result = ruleToFormValues(rule);

      const action = result.action as {
        type: "throttle";
        latency_ms: string;
        download_speed: string;
        upload_speed: string;
      };
      expect(action.type).toBe("throttle");
      expect(action.latency_ms).toBe("500");
      expect(action.download_speed).toBe("100");
      expect(action.upload_speed).toBe("50");
    });

    test("download_rate/upload_rate가 null이면 빈 문자열로 변환된다", () => {
      const rule: InterceptRule = {
        id: "rule-5",
        name: "test",
        enabled: true,
        pattern: "*",
        method: null,
        action: {
          type: "throttle",
          latency_ms: 0,
          download_rate: null,
          upload_rate: null,
        },
      };

      const result = ruleToFormValues(rule);
      const action = result.action as { download_speed: string; upload_speed: string };
      expect(action.download_speed).toBe("");
      expect(action.upload_speed).toBe("");
    });
  });
});

describe("initialValuesToFormValues", () => {
  test("initialValues를 기본 폼 값에 매핑한다", () => {
    const initial: InterceptRuleInitialValues = {
      pattern: "*.example.com/*",
      method: "GET",
    };

    const result = initialValuesToFormValues(initial);

    expect(result.pattern).toBe("*.example.com/*");
    expect(result.method).toBe("GET");
    expect(result.name).toBe("");
    expect(result.action.type).toBe("block");
  });

  test("method가 null이면 '*'로 설정된다", () => {
    const initial: InterceptRuleInitialValues = {
      pattern: "*",
      method: null,
    };

    const result = initialValuesToFormValues(initial);
    expect(result.method).toBe("*");
  });

  test("기본 액션은 block 타입이다", () => {
    const initial: InterceptRuleInitialValues = {
      pattern: "test",
      method: null,
    };

    const result = initialValuesToFormValues(initial);
    expect(result.action).toEqual({
      type: "block",
      status_code: "403",
      body: "",
    });
  });
});

describe("ruleToFormValues → formValuesToAction 왕복 변환", () => {
  test("block 액션의 왕복 변환이 일관성 있다", () => {
    const rule: InterceptRule = {
      id: "r1",
      name: "test",
      enabled: true,
      pattern: "*",
      method: null,
      action: { type: "block", status_code: 403, body: "Blocked" },
    };

    const formValues = ruleToFormValues(rule);
    const action = formValuesToAction(formValues);

    expect(action).toEqual(rule.action);
  });

  test("modify_request 액션의 왕복 변환이 일관성 있다", () => {
    const rule: InterceptRule = {
      id: "r2",
      name: "test",
      enabled: true,
      pattern: "*",
      method: null,
      action: {
        type: "modify_request",
        add_headers: { "X-Key": "value" },
        remove_headers: ["Cookie"],
        set_body: '{"data": 1}',
      },
    };

    const formValues = ruleToFormValues(rule);
    const action = formValuesToAction(formValues);

    expect(action).toEqual(rule.action);
  });

  test("rewrite 액션의 왕복 변환이 일관성 있다", () => {
    const rule: InterceptRule = {
      id: "r3",
      name: "test",
      enabled: true,
      pattern: "*",
      method: null,
      action: {
        type: "rewrite",
        target: "response_body",
        match_pattern: "old",
        replace_with: "new",
      },
    };

    const formValues = ruleToFormValues(rule);
    const action = formValuesToAction(formValues);

    expect(action).toEqual(rule.action);
  });
});
