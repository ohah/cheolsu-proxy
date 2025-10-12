import { useState, useCallback, useRef } from 'react';
import { useForm } from '@tanstack/react-form';

import type { SessionStore } from '@/entities/session';
import { useSessionStore } from '@/shared/stores';
import type { SessionEditFormData } from '../lib/session-edit-schema';

interface UseSessionEditProps {
  session: SessionStore;
}

// 객체 비교를 위한 헬퍼 함수
const isEqual = (a: any, b: any): boolean => {
  // undefined와 null을 같은 것으로 처리
  if (a === undefined || a === null) a = undefined;
  if (b === undefined || b === null) b = undefined;

  if (a === undefined && b === undefined) return true;
  if (a === undefined || b === undefined) return false;

  if (typeof a === 'object' && typeof b === 'object' && a !== null && b !== null) {
    return JSON.stringify(a) === JSON.stringify(b);
  }
  return a === b;
};

/**
 * 세션 편집을 위한 Hook
 * 폼 기반으로 세션 데이터를 편집하고 저장할 수 있습니다.
 */
export const useSessionEdit = ({ session }: UseSessionEditProps) => {
  const { updateSession } = useSessionStore();
  const [isEditing, setIsEditing] = useState(false);
  const originalDataRef = useRef<SessionEditFormData | null>(null);

  // 폼 초기값 설정
  const getInitialValues = (): SessionEditFormData => {
    return {
      id: session.id,
      url: session.url,
      method: session.method,
      isActive: session.isActive,
      request: session.request,
      response: session.response,
    };
  };

  const form = useForm({
    defaultValues: getInitialValues(),
    onSubmit: async ({ value }) => {
      console.log('Form submitted with value:', value);
      // 현재 폼 데이터와 원본 데이터를 비교해서 변경된 필드만 추출
      const originalData = originalDataRef.current;
      console.log('Original data:', originalData);
      if (!originalData) return;

      // 변경된 필드만 추출
      const changedFields: Partial<SessionEditFormData> = {};

      if (!isEqual(value.request, originalData.request)) {
        changedFields.request = value.request;
      }

      if (!isEqual(value.response, originalData.response)) {
        changedFields.response = value.response;
      }

      if (!isEqual(value.url, originalData.url)) {
        changedFields.url = value.url;
      }

      if (!isEqual(value.method, originalData.method)) {
        changedFields.method = value.method;
      }

      if (!isEqual(value.isActive, originalData.isActive)) {
        changedFields.isActive = value.isActive;
      }

      // 변경된 필드가 있는 경우에만 저장
      if (Object.keys(changedFields).length > 0) {
        const updatedSession = { ...session, ...changedFields };
        console.log('Updating session:', updatedSession);
        console.log('Changed fields:', changedFields);
        updateSession(updatedSession as any);
        setIsEditing(false);
      } else {
        console.log('No changes detected');
        setIsEditing(false);
      }
    },
  });

  const startEditing = useCallback(() => {
    const initialValues = getInitialValues();
    originalDataRef.current = initialValues;
    form.setFieldValue('id', initialValues.id);
    form.setFieldValue('url', initialValues.url);
    form.setFieldValue('method', initialValues.method);
    form.setFieldValue('isActive', initialValues.isActive);
    form.setFieldValue('request', initialValues.request);
    form.setFieldValue('response', initialValues.response);
    setIsEditing(true);
  }, [form]);

  const cancelEditing = useCallback(() => {
    setIsEditing(false);
    originalDataRef.current = null;
  }, []);

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
