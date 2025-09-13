import type { TableCellProps } from '../../model';

export const TimeCell = ({ data }: TableCellProps) => {
  const { timeDiff } = data;

  const displayTime = typeof timeDiff === 'number' ? `${timeDiff}ms` : timeDiff;

  return (
    <div className="col-span-1 text-sm font-mono" title={`Response time: ${displayTime}`}>
      {displayTime}
    </div>
  );
};
