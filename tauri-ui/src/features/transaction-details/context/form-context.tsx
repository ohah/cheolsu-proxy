import { createFormHook, createFormHookContexts } from '@tanstack/react-form';
import { TextField, NumberField, TextareaField, SaveButton, CancelButton } from '../ui/form-components';

const { fieldContext, formContext, useFormContext } = createFormHookContexts();

// Create form hook with pre-bound components
export const { useAppForm, withForm } = createFormHook({
  fieldComponents: {
    TextField,
    NumberField,
    TextareaField,
  },
  formComponents: {
    SaveButton,
    CancelButton,
  },
  fieldContext,
  formContext,
});

export { formContext, fieldContext, useFormContext };

export type AppFormInstance = ReturnType<typeof useAppForm>;
