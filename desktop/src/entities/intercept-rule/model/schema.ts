import { z } from "zod";
import { headerPairSchema } from "./common-schemas";

const blockActionSchema = z.object({
  type: z.literal("block"),
  status_code: z.string(),
  body: z.string(),
});

const modifyRequestActionSchema = z.object({
  type: z.literal("modify_request"),
  headers: z.array(headerPairSchema),
  remove_headers: z.array(z.string()),
  body: z.string(),
});

const modifyResponseActionSchema = z.object({
  type: z.literal("modify_response"),
  response_status: z.string(),
  headers: z.array(headerPairSchema),
  remove_headers: z.array(z.string()),
  body: z.string(),
});

const rewriteActionSchema = z.object({
  type: z.literal("rewrite"),
  rewrite_target: z.enum(["request_header", "response_header", "request_body", "response_body"]),
  match_pattern: z.string().min(1, "Match pattern is required"),
  replace_with: z.string(),
});

const throttleActionSchema = z.object({
  type: z.literal("throttle"),
  latency_ms: z.string(),
  download_speed: z.string(),
  upload_speed: z.string(),
});

const actionSchema = z.discriminatedUnion("type", [
  blockActionSchema,
  modifyRequestActionSchema,
  modifyResponseActionSchema,
  rewriteActionSchema,
  throttleActionSchema,
]);

export const interceptRuleFormSchema = z.object({
  name: z.string(),
  pattern: z.string().min(1, "Pattern is required"),
  method: z.string(),
  action: actionSchema,
});

export type InterceptRuleFormValues = z.infer<typeof interceptRuleFormSchema>;

/** 각 액션 타입별로 좁힌 폼 값 타입 */
type BaseFormFields = { name: string; pattern: string; method: string };

export type ModifyRequestFormValues = BaseFormFields & {
  action: z.infer<typeof modifyRequestActionSchema>;
};

export type ModifyResponseFormValues = BaseFormFields & {
  action: z.infer<typeof modifyResponseActionSchema>;
};

export type RewriteFormValues = BaseFormFields & {
  action: z.infer<typeof rewriteActionSchema>;
};

export type ThrottleFormValues = BaseFormFields & {
  action: z.infer<typeof throttleActionSchema>;
};

export const defaultInterceptRuleFormValues: InterceptRuleFormValues = {
  name: "",
  pattern: "",
  method: "*",
  action: {
    type: "block",
    status_code: "403",
    body: "",
  },
};
