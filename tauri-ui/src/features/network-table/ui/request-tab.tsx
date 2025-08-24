import type { ProxiedRequest } from '../../../entities/request';

interface RequestTabProps {
  request: ProxiedRequest;
}

export const RequestTab = ({ request }: RequestTabProps) => {
  const renderBody = () => {
    try {
      const text = new TextDecoder('utf-8', { fatal: true }).decode(request.body);
      return <p>{text}</p>;
    } catch (e) {
      return <p>{`Binary data (bytes): ${Array.from(request.body).join(', ')}`}</p>;
    }
  };

  return (
    <div className="tab-view">
      <div>
        <strong>Properties</strong>
        <div className="headers">
          <div className="single_header">
            <strong>Method:</strong>
            <p>{request.method}</p>
          </div>
          <div className="single_header">
            <strong>Version:</strong>
            <p>{request.version}</p>
          </div>
          <div className="single_header">
            <strong>Timestamp:</strong>
            <p>{new Date(request.time / 1_000_000).toISOString()}</p>
          </div>
        </div>
      </div>
      <div>
        <strong>Headers</strong>
        <div className="headers">
          {Object.entries(request.headers).map(([key, value]) => (
            <div key={key} className="single_header">
              <strong>{`${key}:`}</strong>
              <p>{value}</p>
            </div>
          ))}
        </div>
      </div>
      {request.body && request.body.length > 0 && (
        <div>
          <strong>Body</strong>
          <div className="container_body">{renderBody()}</div>
        </div>
      )}
    </div>
  );
};
