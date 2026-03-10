import { z } from "zod";
import { headerPairSchema } from "./common-schemas";

const mapLocalActionSchema = z.object({
  type: z.literal("map_local"),
  file_path: z.string().min(1, "File path is required"),
  status_code: z.string(),
  headers: z.array(headerPairSchema),
});

const mapRemoteActionSchema = z.object({
  type: z.literal("map_remote"),
  target_url: z.string().min(1, "Target URL is required"),
  preserve_path: z.boolean(),
});

const mapActionSchema = z.discriminatedUnion("type", [
  mapLocalActionSchema,
  mapRemoteActionSchema,
]);

export const mapRuleFormSchema = z.object({
  name: z.string(),
  pattern: z.string().min(1, "Pattern is required"),
  method: z.string(),
  action: mapActionSchema,
});

export type MapRuleFormValues = z.infer<typeof mapRuleFormSchema>;

/** 각 액션 타입별로 좁힌 폼 값 타입 */
type BaseMapFormFields = { name: string; pattern: string; method: string };

export type MapLocalFormValues = BaseMapFormFields & {
  action: z.infer<typeof mapLocalActionSchema>;
};

export type MapRemoteFormValues = BaseMapFormFields & {
  action: z.infer<typeof mapRemoteActionSchema>;
};

export const defaultMapRuleFormValues: MapRuleFormValues = {
  name: "",
  pattern: "",
  method: "*",
  action: {
    type: "map_local",
    file_path: "",
    status_code: "200",
    headers: [],
  },
};
