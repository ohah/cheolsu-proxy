import { z } from 'zod';

// 세션 편집을 위한 스키마
export const sessionEditSchema = z.object({
  id: z.string().min(1, 'ID is required'),
  url: z.string().url('Valid URL is required'),
  method: z.enum(['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS', 'CONNECT', 'TRACE', 'OTHERS']),
  isActive: z.boolean(),
  request: z
    .object({
      headers: z.record(z.string(), z.string()).optional(),
      data: z.union([z.record(z.string(), z.any()), z.string()]).optional(),
      params: z.union([z.record(z.string(), z.any()), z.string()]).optional(),
    })
    .optional(),
  response: z
    .object({
      status: z.number().min(100).max(599),
      headers: z.record(z.string(), z.string()).optional(),
      data: z.union([z.record(z.string(), z.any()), z.string()]).optional(),
    })
    .optional(),
});

export type SessionEditFormData = z.infer<typeof sessionEditSchema>;
