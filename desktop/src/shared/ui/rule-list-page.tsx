import type { ReactNode } from "react";
import { Trans } from "@lingui/react/macro";
import { Card, CardContent, Badge, Button } from "@/shared/ui";
import { Plus, Eraser } from "lucide-react";

export interface RuleListPageProps<T> {
  /** Page title */
  title: ReactNode;
  /** Page description */
  description: ReactNode;
  /** Badge count label (e.g., "rules", "mappings") */
  badgeLabel: ReactNode;
  /** Empty state title */
  emptyTitle: ReactNode;
  /** Empty state description */
  emptyDescription: ReactNode;
  /** Empty state add button label */
  emptyAddLabel: ReactNode;
  /** Add button label in header */
  addLabel: ReactNode;
  /** List of items */
  items: T[];
  /** Get unique key for each item */
  getItemKey: (item: T) => string | number;
  /** Called when "Add" button is clicked */
  onAdd: () => void;
  /** Called when "Clear All" button is clicked */
  onClearAll: () => void;
  /** Render each item's card content */
  renderItem: (item: T) => ReactNode;
  /** Optional content inserted between header and list (e.g., inline form) */
  headerExtra?: ReactNode;
  /** Optional content rendered after the list (e.g., dialogs/modals) */
  dialogs?: ReactNode;
}

export function RuleListPage<T>({
  title,
  description,
  badgeLabel,
  emptyTitle,
  emptyDescription,
  emptyAddLabel,
  addLabel,
  items,
  getItemKey,
  onAdd,
  onClearAll,
  renderItem,
  headerExtra,
  dialogs,
}: RuleListPageProps<T>) {
  return (
    <>
      <div className="flex-1 flex flex-col h-full overflow-auto">
        <div className="p-6 space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-2xl font-bold text-foreground">{title}</h1>
              <p className="text-muted-foreground">{description}</p>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="text-sm">
                {items.length} {badgeLabel}
              </Badge>
              {items.length > 0 && (
                <Button variant="outline" size="sm" onClick={onClearAll}>
                  <Eraser className="w-4 h-4 mr-1" />
                  <Trans>Clear All</Trans>
                </Button>
              )}
              <Button size="sm" onClick={onAdd}>
                <Plus className="w-4 h-4 mr-1" />
                {addLabel}
              </Button>
            </div>
          </div>

          {headerExtra}

          {items.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <div className="text-center space-y-2">
                  <h2 className="text-lg font-semibold">{emptyTitle}</h2>
                  <p className="text-muted-foreground">{emptyDescription}</p>
                  <Button className="mt-4" onClick={onAdd}>
                    <Plus className="w-4 h-4 mr-1" />
                    {emptyAddLabel}
                  </Button>
                </div>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-3">
              {items.map((item) => (
                <Card key={getItemKey(item)}>
                  <CardContent className="py-4">{renderItem(item)}</CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>
      </div>

      {dialogs}
    </>
  );
}
