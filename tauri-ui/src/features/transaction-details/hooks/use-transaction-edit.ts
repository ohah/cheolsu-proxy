import { useState, useCallback, useEffect, useRef } from 'react';
import { z } from 'zod';

import type { HttpTransaction } from '@/entities/proxy';
import { formatBody } from '../lib';
import { useAppForm } from '../context/form-context';

// 편집 가능한 필드들에 대한 스키마 (Properties 제외)
const transactionEditSchema = z.object({
  headers: z.record(z.string(), z.string()),
  body: z.string(),
  responseStatus: z.number().min(100).max(599),
  responseBody: z.string(),
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

  // 폼 초기값 설정
  const getInitialValues = useCallback((): TransactionEditFormData => {
    const { request, response } = transaction;

    return {
      headers: request?.headers || {},
      body: request?.body ? formatBody(request.body) : '',
      responseStatus: response?.status || 200,
      responseBody: response?.body ? formatBody(response.body) : '',
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

      if (!isEqual(value.headers, originalData.headers)) {
        changedFields.headers = value.headers;
      }

      if (!isEqual(value.body, originalData.body)) {
        changedFields.body = value.body;
      }

      if (!isEqual(value.responseStatus, originalData.responseStatus)) {
        changedFields.responseStatus = value.responseStatus;
      }

      if (!isEqual(value.responseBody, originalData.responseBody)) {
        changedFields.responseBody = value.responseBody;
      }

      // 변경된 필드가 있는 경우에만 저장
      if (Object.keys(changedFields).length > 0) {
        console.log('변경된 필드들:', changedFields);
        // TODO: 실제 저장 로직 구현 - changedFields만 전송
        setIsEditing(false);
      } else {
        console.log('변경된 데이터가 없습니다.');
        setIsEditing(false);
      }
    },
  }) as any;

  const startEditing = useCallback(() => {
    const initialValues = getInitialValues();
    originalDataRef.current = initialValues;
    form.setFieldValue('headers', initialValues.headers);
    form.setFieldValue('body', initialValues.body);
    form.setFieldValue('responseStatus', initialValues.responseStatus);
    form.setFieldValue('responseBody', initialValues.responseBody);
    setIsEditing(true);
  }, [form, getInitialValues]);

  const cancelEditing = useCallback(() => {
    if (originalDataRef.current) {
      form.setFieldValue('headers', originalDataRef.current.headers);
      form.setFieldValue('body', originalDataRef.current.body);
      form.setFieldValue('responseStatus', originalDataRef.current.responseStatus);
      form.setFieldValue('responseBody', originalDataRef.current.responseBody);
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
