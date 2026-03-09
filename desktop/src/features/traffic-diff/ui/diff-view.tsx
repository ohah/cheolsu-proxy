import type {
  TrafficDiff,
  TransactionPartDiff,
  HeaderDiff,
  BodyDiff,
  JsonDiffEntry,
  DiffLine,
} from "@/shared/api/proxy";

interface DiffViewProps {
  diff: TrafficDiff;
  onClose: () => void;
}

export const DiffView = ({ diff, onClose }: DiffViewProps) => {
  const hasNoDiff = !diff.request_diff && !diff.response_diff;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
        <h3 className="text-sm font-semibold">Traffic Diff</h3>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground text-sm px-2 py-1 rounded hover:bg-muted"
        >
          Close
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4 text-sm font-mono">
        {hasNoDiff && (
          <p className="text-muted-foreground text-center py-8">No differences found.</p>
        )}

        {diff.request_diff && <PartDiffSection title="Request" part={diff.request_diff} />}

        {diff.response_diff && <PartDiffSection title="Response" part={diff.response_diff} />}
      </div>
    </div>
  );
};

const PartDiffSection = ({ title, part }: { title: string; part: TransactionPartDiff }) => {
  return (
    <div className="space-y-3">
      <h4 className="text-xs font-bold uppercase tracking-wider text-muted-foreground border-b pb-1">
        {title}
      </h4>

      {part.method_diff && (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">Method:</span>
          <span className="text-red-400 line-through">{part.method_diff[0]}</span>
          <span className="text-muted-foreground">&rarr;</span>
          <span className="text-green-400">{part.method_diff[1]}</span>
        </div>
      )}

      {part.url_diff && (
        <div className="space-y-1">
          <span className="text-muted-foreground">URL:</span>
          <div className="text-red-400 bg-red-500/10 px-2 py-0.5 rounded text-xs break-all">
            - {part.url_diff[0]}
          </div>
          <div className="text-green-400 bg-green-500/10 px-2 py-0.5 rounded text-xs break-all">
            + {part.url_diff[1]}
          </div>
        </div>
      )}

      {part.status_diff && (
        <div className="flex items-center gap-2">
          <span className="text-muted-foreground">Status:</span>
          <span className="text-red-400">{part.status_diff[0]}</span>
          <span className="text-muted-foreground">&rarr;</span>
          <span className="text-green-400">{part.status_diff[1]}</span>
        </div>
      )}

      {part.header_diffs.length > 0 && <HeaderDiffSection headers={part.header_diffs} />}

      {part.body_diff && <BodyDiffSection bodyDiff={part.body_diff} />}
    </div>
  );
};

const HeaderDiffSection = ({ headers }: { headers: HeaderDiff[] }) => {
  return (
    <div className="space-y-1">
      <h5 className="text-xs text-muted-foreground font-semibold">Headers</h5>
      <div className="space-y-0.5">
        {headers.map((h, i) => (
          <HeaderDiffRow key={i} diff={h} />
        ))}
      </div>
    </div>
  );
};

const HeaderDiffRow = ({ diff }: { diff: HeaderDiff }) => {
  switch (diff.type) {
    case "added":
      return (
        <div className="text-green-400 bg-green-500/10 px-2 py-0.5 rounded text-xs">
          + {diff.key}: {diff.value}
        </div>
      );
    case "removed":
      return (
        <div className="text-red-400 bg-red-500/10 px-2 py-0.5 rounded text-xs">
          - {diff.key}: {diff.value}
        </div>
      );
    case "modified":
      return (
        <div className="text-yellow-400 bg-yellow-500/10 px-2 py-0.5 rounded text-xs">
          ~ {diff.key}: {diff.old_value} &rarr; {diff.new_value}
        </div>
      );
  }
};

const BodyDiffSection = ({ bodyDiff }: { bodyDiff: BodyDiff }) => {
  return (
    <div className="space-y-1">
      <h5 className="text-xs text-muted-foreground font-semibold">Body</h5>
      {bodyDiff.type === "json" && <JsonDiffView changes={bodyDiff.changes} />}
      {bodyDiff.type === "text" && (
        <TextDiffView
          additions={bodyDiff.additions}
          deletions={bodyDiff.deletions}
          unchanged={bodyDiff.unchanged}
        />
      )}
      {bodyDiff.type === "binary" && (
        <BinaryDiffView oldSize={bodyDiff.old_size} newSize={bodyDiff.new_size} />
      )}
    </div>
  );
};

const JsonDiffView = ({ changes }: { changes: JsonDiffEntry[] }) => {
  if (changes.length === 0) return null;

  return (
    <div className="border rounded p-2 space-y-0.5 bg-muted/20">
      <div className="text-xs text-muted-foreground mb-1">{changes.length} JSON changes</div>
      {changes.map((c, i) => {
        switch (c.change_type) {
          case "added":
            return (
              <div key={i} className="text-green-400 text-xs">
                <span className="text-muted-foreground">{c.path}</span> = {c.new_value}
              </div>
            );
          case "removed":
            return (
              <div key={i} className="text-red-400 text-xs">
                <span className="text-muted-foreground">{c.path}</span> = {c.old_value}
              </div>
            );
          case "modified":
            return (
              <div key={i} className="text-yellow-400 text-xs">
                <span className="text-muted-foreground">{c.path}</span>:{" "}
                <span className="text-red-400">{c.old_value}</span>
                {" \u2192 "}
                <span className="text-green-400">{c.new_value}</span>
              </div>
            );
          default:
            return null;
        }
      })}
    </div>
  );
};

const TextDiffView = ({
  additions,
  deletions,
  unchanged,
}: {
  additions: DiffLine[];
  deletions: DiffLine[];
  unchanged: number;
}) => {
  return (
    <div className="border rounded p-2 space-y-0.5 bg-muted/20">
      <div className="text-xs text-muted-foreground mb-1">
        {unchanged} unchanged, {additions.length} added, {deletions.length} deleted
      </div>
      {deletions.map((d, i) => (
        <div key={`d-${i}`} className="text-red-400 bg-red-500/10 px-2 py-0.5 rounded text-xs">
          - L{d.line_number}: {d.content}
        </div>
      ))}
      {additions.map((a, i) => (
        <div key={`a-${i}`} className="text-green-400 bg-green-500/10 px-2 py-0.5 rounded text-xs">
          + L{a.line_number}: {a.content}
        </div>
      ))}
    </div>
  );
};

const BinaryDiffView = ({ oldSize, newSize }: { oldSize: number; newSize: number }) => {
  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="border rounded p-2 bg-muted/20 text-xs text-muted-foreground">
      Binary content: {formatSize(oldSize)} &rarr; {formatSize(newSize)}
    </div>
  );
};
