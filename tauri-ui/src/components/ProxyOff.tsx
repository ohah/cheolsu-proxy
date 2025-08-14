
import React, { useState } from 'react';
import { startProxy } from '../api';
import TextInput from './TextInput';

interface ProxyOffProps {
  onStart: () => void;
}

const ProxyOff: React.FC<ProxyOffProps> = ({ onStart }) => {
  const [proxyAddr, setProxyAddr] = useState('127.0.0.1:8100');
  const [error, setError] = useState<string | null>(null);

  // A simple regex for IP:PORT format. For a real app, a more robust validation would be better.
  const validateAddress = (addr: string) => {
    const pattern = /^(\d{1,3}\.){3}\d{1,3}:\d{1,5}$/;
    return pattern.test(addr);
  };

  const handleAddrChange = (newAddr: string) => {
    setProxyAddr(newAddr);
    if (!validateAddress(newAddr)) {
      setError('Invalid address format. Use IP:PORT (e.g., 127.0.0.1:8100)');
    } else {
      setError(null);
    }
  };

  const handleStartClick = async () => {
    if (!validateAddress(proxyAddr)) {
        setError('Cannot start with an invalid address.');
        return;
    }
    try {
      await startProxy(proxyAddr);
      onStart();
      setError(null);
    } catch (err) {
      console.error(err);
      setError(`Failed to start proxy: ${err}`);
    }
  };

  return (
    <div className="proxy-off">
      <TextInput value={proxyAddr} onChange={handleAddrChange} />
      {error && <p className="error">{error}</p>}
      <button onClick={handleStartClick} disabled={!!error}>▶</button>
    </div>
  );
};

export default ProxyOff;
