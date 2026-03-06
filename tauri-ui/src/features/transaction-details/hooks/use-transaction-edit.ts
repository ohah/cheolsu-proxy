import { useState, useCallback, useEffect, useRef } from "react";
import { z } from "zod";

import type { HttpTransaction } from "@/entities/proxy";
import { useAppForm } from "../context/form-context";
import { formatBodyContent } from "../lib";

// 편집 가능한 필드들에 대한 스키마
const transactionEditSchema = z.object({
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

export type TransactionEditFormData = z.infer<typeof transactionEditSchema>;

export const useTransactionEdit = (transaction: HttpTransaction) => {
  const [isEditing, setIsEditing] = useState(false);
  const originalDataRef = useRef<TransactionEditFormData | null>(null);

  // 다른 요청이 선택되면 편집 모드 자동 종료
  useEffect(() => {
    if (isEditing) {
      setIsEditing(false);
      originalDataRef.current = null;
    }
  }, [transaction.request?.id, transaction.request?.time]); // transaction이 변경될 때

  // 폼 초기값 설정 (세션 스토어 타입과 일치)
  const getInitialValues = useCallback((): TransactionEditFormData => {
    const { request, response } = transaction;

    return {
      request: {
        ...request,
        headers: request?.headers,
        data: request?.body
          ? formatBodyContent(request.body, request.data_type, request.body_json)
          : "",
      },
      response: {
        ...response,
        status: response?.status || 200,
        headers: response?.headers,
        data: response?.body
          ? formatBodyContent(response.body, response.data_type, response.body_json)
          : "",
      },
    };
  }, [transaction]);

  const form = useAppForm({
    defaultValues: getInitialValues(),
    validators: {
      onChange: transactionEditSchema,
    },
    onSubmit: async () => {
      setIsEditing(false);
    },
  }) as any;

  const startEditing = useCallback(() => {
    const initialValues = getInitialValues();
    originalDataRef.current = initialValues;
    form.setFieldValue("request", initialValues.request);
    form.setFieldValue("response", initialValues.response);
    setIsEditing(true);
  }, [form, getInitialValues]);

  const cancelEditing = useCallback(() => {
    if (originalDataRef.current) {
      form.setFieldValue("request", originalDataRef.current.request);
      form.setFieldValue("response", originalDataRef.current.response);
    }
    setIsEditing(false);
    originalDataRef.current = null;
  }, [form]);

  const saveChanges = useCallback(() => {
    form.handleSubmit();
  }, [form]);

  return {
    isEditing,
    form,
    startEditing,
    cancelEditing,
    saveChanges,
  };
};
