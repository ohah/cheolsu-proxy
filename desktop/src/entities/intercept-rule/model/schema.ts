import { z } from "zod";

const blockActionSchema = z.object({
  type: z.literal("block"),
  status_code: z.string(),
  body: z.string(),
});

const headerPairSchema = z.object({
  key: z.string(),
  value: z.string(),
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
  rewrite_target: z.enum([
    "request_header",
    "response_header",
    "request_body",
    "response_body",
  ]),
  match_pattern: z.string().min(1, "Match pattern is required"),
  replace_with: z.string(),
});

const actionSchema = z.discriminatedUnion("type", [
  blockActionSchema,
  modifyRequestActionSchema,
  modifyResponseActionSchema,
  rewriteActionSchema,
]);

export const interceptRuleFormSchema = z.object({
  name: z.string(),
  pattern: z.string().min(1, "Pattern is required"),
  method: z.string(),
  action: actionSchema,
});

export type InterceptRuleFormValues = z.infer<typeof interceptRuleFormSchema>;

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
