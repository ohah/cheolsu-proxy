
import React, { useState } from 'react';
import { stopProxy } from '../api';
import RequestTable from './request/RequestTable';

interface ProxyOnProps {
  onStop: () => void;
}

const ProxyOn: React.FC<ProxyOnProps> = ({ onStop }) => {
  const [paused, setPaused] = useState(false);

  const handleStopClick = async () => {
    try {
      await stopProxy();
      onStop();
    } catch (err) {
      console.error('Failed to stop proxy:', err);
      // Optionally, display an error to the user
    }
  };

  return (
    <div className="proxy-on">
      <div className="controls">
        <button onClick={() => setPaused(!paused)}>{paused ? '▶' : '⏸'}</button>
        <button onClick={handleStopClick}>⏹</button>
      </div>
      <RequestTable paused={paused} />
    </div>
  );
};

export default ProxyOn;
