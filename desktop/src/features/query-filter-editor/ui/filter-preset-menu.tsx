import { useState, useCallback } from "react";
import { Bookmark, Plus, Trash2 } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { useLingui } from "@lingui/react/macro";

import { cn } from "@/shared/lib";
import {
  Input,
  Popover,
  PopoverTrigger,
  PopoverContent,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/shared/ui";
import { useFilterPresetStore, type FilterPreset } from "@/shared/stores/filter-preset-store";

interface FilterPresetMenuProps {
  currentQuery: string;
  onSelectPreset: (query: string) => void;
}

export function FilterPresetMenu({ currentQuery, onSelectPreset }: FilterPresetMenuProps) {
  const { t } = useLingui();
  const presets = useFilterPresetStore((s) => s.presets);
  const addPreset = useFilterPresetStore((s) => s.addPreset);
  const deletePreset = useFilterPresetStore((s) => s.deletePreset);

  const [open, setOpen] = useState(false);
  const [isAdding, setIsAdding] = useState(false);
  const [newName, setNewName] = useState("");

  const handleSave = useCallback(() => {
    const name = newName.trim();
    if (!name || !currentQuery.trim()) return;
    addPreset({ name, query: currentQuery });
    setNewName("");
    setIsAdding(false);
  }, [newName, currentQuery, addPreset]);

  const handleSelect = useCallback(
    (preset: FilterPreset) => {
      onSelectPreset(preset.query);
      setOpen(false);
    },
    [onSelectPreset],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSave();
      } else if (e.key === "Escape") {
        setIsAdding(false);
        setNewName("");
      }
    },
    [handleSave],
  );

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Tooltip open={open ? false : undefined}>
        <TooltipTrigger
          render={
            <PopoverTrigger
              className={cn(
                "p-1.5 hover:bg-accent/10 rounded-md transition-colors",
                presets.length > 0 && "text-accent",
              )}
            />
          }
        >
          <Bookmark className="w-3.5 h-3.5 text-muted-foreground" />
        </TooltipTrigger>
        <TooltipContent side="bottom">{t`Filter Presets`}</TooltipContent>
      </Tooltip>
      <PopoverContent align="end" className="w-64 p-2">
        <div className="text-xs font-medium text-muted-foreground px-2 py-1">
          <Trans>Filter Presets</Trans>
        </div>

        {presets.length === 0 && !isAdding && (
          <div className="text-xs text-muted-foreground/60 px-2 py-3 text-center">
            <Trans>No saved presets</Trans>
          </div>
        )}

        <div className="max-h-48 overflow-y-auto">
          {presets.map((preset) => (
            <div
              key={preset.id}
              className="flex items-center gap-1 group rounded-md hover:bg-muted px-2 py-1.5 cursor-pointer"
              onClick={() => handleSelect(preset)}
            >
              <div className="flex-1 min-w-0">
                <div className="text-sm truncate">{preset.name}</div>
                <div className="text-[10px] text-muted-foreground font-mono truncate">
                  {preset.query}
                </div>
              </div>
              <button
                type="button"
                className="opacity-0 group-hover:opacity-100 p-1 hover:text-destructive transition-all"
                onClick={(e) => {
                  e.stopPropagation();
                  deletePreset(preset.id);
                }}
                title={t`Delete`}
              >
                <Trash2 className="w-3 h-3" />
              </button>
            </div>
          ))}
        </div>

        {isAdding ? (
          <div className="flex items-center gap-1 mt-1 px-1">
            <Input
              type="text"
              className="flex-1 h-7 text-sm"
              placeholder={t`Preset name`}
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={handleKeyDown}
              autoFocus
            />
            <button
              type="button"
              className="text-xs text-accent hover:text-accent/80 px-2 py-1"
              onClick={handleSave}
            >
              <Trans>Save</Trans>
            </button>
          </div>
        ) : (
          <button
            type="button"
            className="w-full flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground px-2 py-1.5 mt-1 rounded-md hover:bg-muted transition-colors"
            onClick={() => {
              if (!currentQuery.trim()) return;
              setIsAdding(true);
            }}
            disabled={!currentQuery.trim()}
          >
            <Plus className="w-3.5 h-3.5" />
            <Trans>Save current filter</Trans>
          </button>
        )}
      </PopoverContent>
    </Popover>
  );
}
