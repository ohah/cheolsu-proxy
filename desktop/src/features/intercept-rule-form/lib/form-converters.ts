import type { InterceptRule, InterceptAction } from "@/entities/intercept-rule";
import type { InterceptRuleFormValues } from "@/entities/intercept-rule";
import { defaultInterceptRuleFormValues } from "@/entities/intercept-rule";
import type { InterceptRuleInitialValues } from "@/shared/stores";

/**
 * 도메인 모델(InterceptRule)을 폼 값(InterceptRuleFormValues)으로 변환합니다.
 */
export function ruleToFormValues(rule: InterceptRule): InterceptRuleFormValues {
  const base = {
    name: rule.name,
    pattern: rule.pattern,
    method: rule.method ?? "*",
  };

  switch (rule.action.type) {
    case "block":
      return {
        ...base,
        action: {
          type: "block",
          status_code: String(rule.action.status_code),
          body: rule.action.body,
        },
      };
    case "modify_request":
      return {
        ...base,
        action: {
          type: "modify_request",
          headers: Object.entries(rule.action.add_headers).map(([key, value]) => ({ key, value })),
          remove_headers: rule.action.remove_headers,
          body: rule.action.set_body ?? "",
        },
      };
    case "modify_response":
      return {
        ...base,
        action: {
          type: "modify_response",
          response_status: rule.action.set_status ? String(rule.action.set_status) : "",
          headers: Object.entries(rule.action.add_headers).map(([key, value]) => ({ key, value })),
          remove_headers: rule.action.remove_headers,
          body: rule.action.set_body ?? "",
        },
      };
    case "rewrite":
      return {
        ...base,
        action: {
          type: "rewrite",
          rewrite_target: rule.action.target,
          match_pattern: rule.action.match_pattern,
          replace_with: rule.action.replace_with,
        },
      };
    case "throttle":
      return {
        ...base,
        action: {
          type: "throttle",
          latency_ms: String(rule.action.latency_ms),
          download_speed: rule.action.download_rate ? String(rule.action.download_rate / 1024) : "",
          upload_speed: rule.action.upload_rate ? String(rule.action.upload_rate / 1024) : "",
        },
      };
    default:
      return { ...base, ...defaultInterceptRuleFormValues };
  }
}

/**
 * 폼 값(InterceptRuleFormValues)을 도메인 모델(InterceptAction)로 변환합니다.
 */
export function formValuesToAction(values: InterceptRuleFormValues): InterceptAction {
  const action = values.action;

  switch (action.type) {
    case "block":
      return {
        type: "block",
        status_code: parseInt(action.status_code) || 403,
        body: action.body,
      };
    case "modify_request": {
      const headers: Record<string, string> = {};
      for (const { key, value } of action.headers) {
        const k = key.trim();
        if (k) headers[k] = value;
      }
      return {
        type: "modify_request",
        add_headers: headers,
        remove_headers: action.remove_headers.filter((h) => h.trim()),
        set_body: action.body.trim() || null,
      };
    }
    case "modify_response": {
      const headers: Record<string, string> = {};
      for (const { key, value } of action.headers) {
        const k = key.trim();
        if (k) headers[k] = value;
      }
      return {
        type: "modify_response",
        set_status: action.response_status ? parseInt(action.response_status) || null : null,
        add_headers: headers,
        remove_headers: action.remove_headers.filter((h) => h.trim()),
        set_body: action.body.trim() || null,
      };
    }
    case "rewrite":
      return {
        type: "rewrite",
        target: action.rewrite_target,
        match_pattern: action.match_pattern,
        replace_with: action.replace_with,
      };
    case "throttle": {
      const dlRate = action.download_speed ? parseInt(action.download_speed) * 1024 : null;
      const ulRate = action.upload_speed ? parseInt(action.upload_speed) * 1024 : null;
      return {
        type: "throttle",
        download_rate: dlRate && dlRate > 0 ? dlRate : null,
        upload_rate: ulRate && ulRate > 0 ? ulRate : null,
        latency_ms: parseInt(action.latency_ms) || 0,
      };
    }
    default:
      return { type: "block", status_code: 403, body: "" };
  }
}

/**
 * initialValues에서 기본 폼 값을 생성합니다.
 */
export function initialValuesToFormValues(
  initialValues: InterceptRuleInitialValues,
): InterceptRuleFormValues {
  return {
    ...defaultInterceptRuleFormValues,
    pattern: initialValues.pattern,
    method: initialValues.method ?? "*",
  };
}
