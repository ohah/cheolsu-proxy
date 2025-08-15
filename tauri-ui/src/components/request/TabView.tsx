
import React from 'react';

// Assuming headers are a Record<string, string>
// and body is string or a format that can be displayed.
interface TabViewProps {
  headers: Record<string, string>;
  body: string | Uint8Array;
  children: React.ReactNode;
}

const TabView: React.FC<TabViewProps> = ({ headers, body, children }) => {

  const renderBody = () => {
    if (typeof body === 'string') {
      return <p>{body}</p>;
    }
    // For Uint8Array, you might want to display it as a hex dump or try to decode it as text.
    try {
      const text = new TextDecoder('utf-8', { fatal: true }).decode(body);
      return <p>{text}</p>;
    } catch (e) {
      return <p>{`Binary data (bytes): ${body.join(', ')}`}</p>;
    }
  };

  return (
    <div className="tab-view">
      <div>
        <strong>Properties</strong>
        <div className="headers">{children}</div>
      </div>
      <div>
        <strong>Headers</strong>
        <div className="headers">
          {Object.entries(headers).map(([key, value]) => (
            <div key={key} className="single_header">
              <strong>{`${key}:`}</strong>
              <p>{value}</p>
            </div>
          ))}
        </div>
      </div>
      {body && body.length > 0 && (
        <div>
          <strong>Body</strong>
          <div className="container_body">
            {renderBody()}
          </div>
        </div>
      )}
    </div>
  );
};

export default TabView;
