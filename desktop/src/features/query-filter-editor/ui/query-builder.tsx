import { useCallback } from "react";
import { Plus, X } from "lucide-react";
import { Trans, useLingui } from "@lingui/react/macro";

import { cn } from "@/shared/lib";
import {
  Button,
  Badge,
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
  Input,
} from "@/shared/ui";
import { Separator } from "@/shared/ui/separator";

import { HTTP_METHODS, STATUS_CODES } from "../model/constants";
import type { FilterCondition, BuilderState } from "../lib/query-serializer";
import { createEmptyCondition, serializeBuilderState } from "../lib/query-serializer";

type FieldType = FilterCondition["field"];
type OperatorType = FilterCondition["operator"];

const FIELD_OPTIONS: { value: FieldType; label: string }[] = [
  { value: "method", label: "Method" },
  { value: "status", label: "Status" },
  { value: "url", label: "URL" },
];

function getOperatorsForField(field: FieldType): { value: OperatorType; label: string }[] {
  if (field === "url") {
    return [
      { value: "|=", label: "|= contains" },
      { value: "=", label: "= equals" },
      { value: "!=", label: "!= exclude" },
    ];
  }
  return [
    { value: "=", label: "= include" },
    { value: "!=", label: "!= exclude" },
  ];
}

interface ValueChipSelectorProps {
  options: readonly { value: string; label: string }[];
  selected: string[];
  onToggle: (value: string) => void;
}

const ValueChipSelector = ({ options, selected, onToggle }: ValueChipSelectorProps) => {
  return (
    <div className="flex flex-wrap gap-1">
      {options.map((opt) => {
        const isSelected = selected.includes(opt.value);
        return (
          <button
            key={opt.value}
            type="button"
            onClick={() => onToggle(opt.value)}
            className={cn(
              "px-2 py-0.5 rounded-md text-xs font-mono border transition-colors cursor-pointer",
              isSelected
                ? "bg-accent text-accent-foreground border-accent"
                : "bg-muted/50 text-muted-foreground border-transparent hover:bg-muted hover:border-border",
            )}
          >
            {opt.value}
            {opt.label !== opt.value && (
              <span className="ml-1 text-[10px] opacity-60">{opt.label}</span>
            )}
          </button>
        );
      })}
    </div>
  );
};

const METHOD_OPTIONS = HTTP_METHODS.map((m) => ({ value: m, label: m }));
const STATUS_OPTIONS = STATUS_CODES.map((s) => ({ value: s.code, label: s.detail }));

interface ConditionRowProps {
  condition: FilterCondition;
  index: number;
  total: number;
  logicalOperator: "and" | "or";
  onUpdate: (id: string, updates: Partial<Omit<FilterCondition, "id">>) => void;
  onRemove: (id: string) => void;
}

const ConditionRow = ({
  condition,
  index,
  total,
  logicalOperator,
  onUpdate,
  onRemove,
}: ConditionRowProps) => {
  const { t } = useLingui();
  const operators = getOperatorsForField(condition.field);

  const handleFieldChange = useCallback(
    (value: string | null) => {
      if (!value) return;
      const newField = value as FieldType;
      const newOps = getOperatorsForField(newField);
      const opStillValid = newOps.some((o) => o.value === condition.operator);
      onUpdate(condition.id, {
        field: newField,
        operator: opStillValid ? condition.operator : newOps[0].value,
        values: [],
      });
    },
    [condition.id, condition.operator, onUpdate],
  );

  const handleOperatorChange = useCallback(
    (value: string | null) => {
      if (!value) return;
      onUpdate(condition.id, { operator: value as OperatorType });
    },
    [condition.id, onUpdate],
  );

  const handleValueToggle = useCallback(
    (value: string) => {
      const current = condition.values;
      const next = current.includes(value) ? current.filter((v) => v !== value) : [...current, value];
      onUpdate(condition.id, { values: next });
    },
    [condition.id, condition.values, onUpdate],
  );

  const handleUrlInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const raw = e.target.value;
      onUpdate(condition.id, { values: raw ? [raw] : [] });
    },
    [condition.id, onUpdate],
  );

  return (
    <div className="flex items-start gap-2 group">
      {/* Row connector label */}
      <div className="w-10 shrink-0 flex items-center justify-center h-8">
        {index === 0 ? (
          <span className="text-xs text-muted-foreground font-medium">
            <Trans>Where</Trans>
          </span>
        ) : (
          <span className="text-xs text-accent font-mono font-medium uppercase">
            {logicalOperator}
          </span>
        )}
      </div>

      {/* Field select */}
      <Select value={condition.field} onValueChange={handleFieldChange}>
        <SelectTrigger size="sm" className="w-24 shrink-0">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {FIELD_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value} label={opt.label}>
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Operator select */}
      <Select value={condition.operator} onValueChange={handleOperatorChange}>
        <SelectTrigger size="sm" className="w-32 shrink-0 font-mono">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {operators.map((opt) => (
            <SelectItem key={opt.value} value={opt.value} label={opt.label}>
              <span className="font-mono">{opt.label}</span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Value selector */}
      <div className="flex-1 min-w-0">
        {condition.field === "method" && (
          <ValueChipSelector
            options={METHOD_OPTIONS}
            selected={condition.values}
            onToggle={handleValueToggle}
          />
        )}
        {condition.field === "status" && (
          <ValueChipSelector
            options={STATUS_OPTIONS}
            selected={condition.values}
            onToggle={handleValueToggle}
          />
        )}
        {condition.field === "url" && (
          <Input
            className="h-8 text-xs font-mono"
            placeholder={t`e.g. api.example.com, /api/v1`}
            value={condition.values[0] ?? ""}
            onChange={handleUrlInput}
          />
        )}
      </div>

      {/* Remove button */}
      <Button
        variant="ghost"
        size="sm"
        className="h-8 w-8 p-0 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={() => onRemove(condition.id)}
        disabled={total <= 1}
      >
        <X className="w-3.5 h-3.5" />
      </Button>
    </div>
  );
};

export interface QueryBuilderProps {
  builderState: BuilderState;
  onBuilderStateChange: (state: BuilderState) => void;
  onApply: (query: string) => void;
  totalCount: number;
  filteredCount: number;
}

export const QueryBuilder = ({
  builderState,
  onBuilderStateChange,
  onApply,
  totalCount,
  filteredCount,
}: QueryBuilderProps) => {
  const handleUpdateCondition = useCallback(
    (id: string, updates: Partial<Omit<FilterCondition, "id">>) => {
      const next = {
        ...builderState,
        conditions: builderState.conditions.map((c) => (c.id === id ? { ...c, ...updates } : c)),
      };
      onBuilderStateChange(next);
    },
    [builderState, onBuilderStateChange],
  );

  const handleRemoveCondition = useCallback(
    (id: string) => {
      const next = {
        ...builderState,
        conditions: builderState.conditions.filter((c) => c.id !== id),
      };
      onBuilderStateChange(next);
    },
    [builderState, onBuilderStateChange],
  );

  const handleAddCondition = useCallback(() => {
    const next = {
      ...builderState,
      conditions: [...builderState.conditions, createEmptyCondition()],
    };
    onBuilderStateChange(next);
  }, [builderState, onBuilderStateChange]);

  const handleToggleOperator = useCallback(() => {
    const next = {
      ...builderState,
      logicalOperator: (builderState.logicalOperator === "and" ? "or" : "and") as "and" | "or",
    };
    onBuilderStateChange(next);
  }, [builderState, onBuilderStateChange]);

  const handleApply = useCallback(() => {
    const query = serializeBuilderState(builderState);
    onApply(query);
  }, [builderState, onApply]);

  const handleClear = useCallback(() => {
    const next: BuilderState = {
      conditions: [createEmptyCondition()],
      logicalOperator: "and",
    };
    onBuilderStateChange(next);
    onApply("");
  }, [onBuilderStateChange, onApply]);

  const statsText =
    totalCount !== filteredCount ? `${filteredCount} / ${totalCount}` : `${totalCount}`;

  const hasAnyValue = builderState.conditions.some((c) => c.values.length > 0);

  return (
    <div className="w-full border rounded-md bg-background">
      {/* Conditions */}
      <div className="p-3 space-y-2">
        {builderState.conditions.map((condition, index) => (
          <ConditionRow
            key={condition.id}
            condition={condition}
            index={index}
            total={builderState.conditions.length}
            logicalOperator={builderState.logicalOperator}
            onUpdate={handleUpdateCondition}
            onRemove={handleRemoveCondition}
          />
        ))}
      </div>

      <Separator />

      {/* Footer actions */}
      <div className="px-3 py-2 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" onClick={handleAddCondition}>
            <Plus className="w-3.5 h-3.5" />
            <Trans>Condition</Trans>
          </Button>

          {builderState.conditions.length > 1 && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs font-mono px-2"
              onClick={handleToggleOperator}
            >
              {builderState.logicalOperator.toUpperCase()}
            </Button>
          )}
        </div>

        <div className="flex items-center gap-2">
          <Badge className="text-[10px] font-mono shrink-0 bg-accent text-accent-foreground">
            {statsText}
          </Badge>

          {hasAnyValue && (
            <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={handleClear}>
              <Trans>Clear</Trans>
            </Button>
          )}

          <Button size="sm" className="h-7 text-xs" onClick={handleApply}>
            <Trans>Apply</Trans>
          </Button>
        </div>
      </div>
    </div>
  );
};
