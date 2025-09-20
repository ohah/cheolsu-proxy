import { useState, useCallback, useEffect, useRef } from 'react';
import { z } from 'zod';

import type { HttpTransaction } from '@/entities/proxy';
import { useAppForm } from '../context/form-context';
import { formatBody } from '../lib';
import { useSessionStore } from '@/shared/stores';

// 편집 가능한 필드들에 대한 스키마 (세션 스토어 타입과 일치)
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

// 객체 비교를 위한 헬퍼 함수
const isEqual = (a: any, b: any): boolean => {
  if (typeof a !== typeof b) return false;
  if (typeof a === 'object' && a !== null && b !== null) {
    return JSON.stringify(a) === JSON.stringify(b);
  }
  return a === b;
};

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
        data: request?.body ? formatBody(request.body) : '',
      },
      response: {
        ...response,
        status: response?.status || 200,
        headers: response?.headers,
        data: response?.body ? formatBody(response.body) : '',
      },
    };
  }, [transaction]);

  const form = useAppForm({
    defaultValues: getInitialValues(),
    validators: {
      onChange: transactionEditSchema,
    },
    onSubmit: async ({ value }) => {
      // 현재 폼 데이터와 원본 데이터를 비교해서 변경된 필드만 추출
      const originalData = originalDataRef.current;
      if (!originalData) return;

      // 변경된 필드만 추출
      const changedFields: Partial<TransactionEditFormData> = {};

      delete (changedFields.response as any)?.body;
      delete (changedFields.request as any)?.body;

      if (!isEqual(value.request, originalData.request)) {
        changedFields.request = value.request;
      }

      if (!isEqual(value.response, originalData.response)) {
        changedFields.response = value.response;
      }

      const saveData = {
        id: crypto.randomUUID(),
        url: transaction.request?.uri || '',
        method: transaction.request?.method || 'GET',
        request: changedFields.request,
        response: {
          headers: changedFields.response?.headers,
          status: changedFields.response?.status,
          data: changedFields.response?.data,
        },
      };

      // 변경된 필드가 있는 경우에만 저장
      if (Object.keys(changedFields).length > 0) {
        await useSessionStore.getState().addSession(saveData as any);
        setIsEditing(false);
      } else {
        setIsEditing(false);
      }
    },
  }) as any;

  const startEditing = useCallback(() => {
    const initialValues = getInitialValues();
    originalDataRef.current = initialValues;
    form.setFieldValue('request', initialValues.request);
    form.setFieldValue('response', initialValues.response);
    setIsEditing(true);
  }, [form, getInitialValues]);

  const cancelEditing = useCallback(() => {
    if (originalDataRef.current) {
      form.setFieldValue('request', originalDataRef.current.request);
      form.setFieldValue('response', originalDataRef.current.response);
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
