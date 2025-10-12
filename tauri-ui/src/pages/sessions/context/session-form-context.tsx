import { createContext, useContext } from 'react';
import { useForm } from '@tanstack/react-form';
import { zodValidator } from '@tanstack/zod-form-adapter';

import type { SessionEditFormData } from '../lib/session-edit-schema';

export interface SessionFormInstance {
  getFieldValue: (name: string) => any;
  setFieldValue: (name: string, value: any) => void;
  validate: () => Promise<boolean>;
  reset: () => void;
  getValues: () => SessionEditFormData;
}

const SessionFormContext = createContext<SessionFormInstance | null>(null);

export const useSessionForm = () => {
  const context = useContext(SessionFormContext);
  if (!context) {
    throw new Error('useSessionForm must be used within a SessionFormProvider');
  }
  return context;
};

export { SessionFormContext };
