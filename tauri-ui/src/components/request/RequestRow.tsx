
import React from 'react';
import { RequestInfo } from '../../types';

interface RequestRowProps {
  exchange: RequestInfo;
  idx: number;
  onDelete: (id: number) => void;
  onSelect: (id: number) => void;
}

const RequestRow: React.FC<RequestRowProps> = ({ exchange, idx, onDelete, onSelect }) => {
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
      return url.hostname + (url.port ? ':' + url.port : '');
    } catch (e) {
      return uri.split('/')[0] || uri;
    }
  }

  const timeDiff = response.time && request.time ?
    Math.trunc((response.time - request.time) / 1e6) : 'N/A';

  return (
    <tr className="grid-body" onClick={handleSelect}>
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
        <button title="Delete" className="delete-btn" onClick={handleDelete}>
          🗑
        </button>
      </td>
    </tr>
  );
};

export default RequestRow;
