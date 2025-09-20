import { Input, Textarea, Button } from '@/shared/ui';
import type { FieldApi } from '@tanstack/react-form';

// Form Field Components
export const TextField = ({
  field,
  label,
  placeholder,
}: {
  field: FieldApi<
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any
  >;
  label?: string;
  placeholder?: string;
}) => (
  <div className="space-y-2">
    {label && <label className="text-sm font-medium">{label}</label>}
    <Input
      value={field.state.value || ''}
      onChange={(e) => field.handleChange(e.target.value)}
      onBlur={field.handleBlur}
      placeholder={placeholder}
      className="font-mono text-xs"
    />
  </div>
);

export const NumberField = ({
  field,
  label,
  placeholder,
  min,
  max,
}: {
  field: FieldApi<
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any
  >;
  label?: string;
  placeholder?: string;
  min?: number;
  max?: number;
}) => (
  <div className="space-y-2">
    {label && <label className="text-sm font-medium">{label}</label>}
    <Input
      type="number"
      value={field.state.value || ''}
      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
      onBlur={field.handleBlur}
      placeholder={placeholder}
      min={min}
      max={max}
      className="font-mono text-xs"
    />
  </div>
);

export const TextareaField = ({
  field,
  label,
  placeholder,
  minHeight = 200,
}: {
  field: FieldApi<
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any,
    any
  >;
  label?: string;
  placeholder?: string;
  minHeight?: number;
}) => (
  <div className="space-y-2">
    {label && <label className="text-sm font-medium">{label}</label>}
    <Textarea
      value={field.state.value || ''}
      onChange={(e) => field.handleChange(e.target.value)}
      onBlur={field.handleBlur}
      placeholder={placeholder}
      className={`font-mono text-xs`}
      style={{ minHeight: `${minHeight}px` }}
    />
  </div>
);

// Form Components
export const SaveButton = ({ form }: { form: any }) => (
  <Button type="submit" variant="default" size="sm" disabled={form.state.isSubmitting}>
    Save
  </Button>
);

export const CancelButton = ({ onClick }: { onClick: () => void }) => (
  <Button type="button" variant="ghost" size="sm" onClick={onClick}>
    Cancel
  </Button>
);
