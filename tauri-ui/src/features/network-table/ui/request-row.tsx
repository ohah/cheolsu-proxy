import type { RequestInfo } from '../../../entities/request';

interface RequestRowProps {
  exchange: RequestInfo;
  idx: number;
  onDelete: (id: number) => void;
  onSelect: (id: number) => void;
}

export const RequestRow = ({ exchange, idx, onDelete, onSelect }: RequestRowProps) => {
  const { request, response } = exchange;

  if (!request || !response) {
    return (
      <tr>
        <td colSpan={6}>Parsing Failed</td>
      </tr>
    );
  }

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent row selection
    onDelete(idx);
  };

  const handleSelect = () => {
    onSelect(idx);
  };

  const getAuthority = (uri: string) => {
    try {
      const url = new URL(uri);
      return `${url.hostname}${url.port ? `:${url.port}` : ''}`;
    } catch (e) {
      return uri.split('/')[0] || uri;
    }
  };

  const timeDiff = response.time && request.time ? Math.trunc((response.time - request.time) / 1e6) : 'N/A';

  return (
    <tr
      className="grid-body"
      onClick={handleSelect}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          handleSelect();
        }
      }}
      tabIndex={0}
    >
      <td className="path">
        <b>{getAuthority(request.uri)}</b>
        <br />
        <span>{new URL(request.uri).pathname}</span>
      </td>
      <td className={`method ${request.method}`}>{request.method}</td>
      <td>{response.status}</td>
      <td>{request.body.length}</td>
      <td>{timeDiff} ms</td>
      <td>
        <button title="Delete" className="delete-btn" onClick={handleDelete} type="button">
          🗑
        </button>
      </td>
    </tr>
  );
};
