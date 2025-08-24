import type { ProxiedResponse } from '../../../entities/request';

interface ResponseTabProps {
  response: ProxiedResponse;
}

export const ResponseTab = ({ response }: ResponseTabProps) => {
  const renderBody = () => {
    try {
      const text = new TextDecoder('utf-8', { fatal: true }).decode(response.body);
      // JSON인 경우 포맷팅
      try {
        const json = JSON.parse(text);
        return <p>{JSON.stringify(json, null, 2)}</p>;
      } catch {
        return <p>{text}</p>;
      }
    } catch (e) {
      return <p>{`Binary data (bytes): ${Array.from(response.body).join(', ')}`}</p>;
    }
  };

  return (
    <div className="tab-view">
      <div>
        <strong>Properties</strong>
        <div className="headers">
          <div className="single_header">
            <strong>Status:</strong>
            <p>{response.status}</p>
          </div>
          <div className="single_header">
            <strong>Version:</strong>
            <p>{response.version}</p>
          </div>
          <div className="single_header">
            <strong>Timestamp:</strong>
            <p>{new Date(response.time / 1_000_000).toISOString()}</p>
          </div>
        </div>
      </div>
      <div>
        <strong>Headers</strong>
        <div className="headers">
          {Object.entries(response.headers).map(([key, value]) => (
            <div key={key} className="single_header">
              <strong>{`${key}:`}</strong>
              <p>{value}</p>
            </div>
          ))}
        </div>
      </div>
      {response.body && response.body.length > 0 && (
        <div>
          <strong>Body</strong>
          <div className="container_body">{renderBody()}</div>
        </div>
      )}
    </div>
  );
};
