import { z } from "zod";

export const headerPairSchema = z.object({
  key: z.string(),
  value: z.string(),
});
