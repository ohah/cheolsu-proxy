
import React, { useState, useEffect } from 'react';

const ThemeButton: React.FC = () => {
  const [isDark, setIsDark] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    setIsDark(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches);
    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  useEffect(() => {
    document.body.setAttribute('data-theme', isDark ? 'dark' : 'light');
  }, [isDark]);

  const toggleTheme = () => {
    setIsDark(!isDark);
  };

  return <button onClick={toggleTheme}>{isDark ? '🔆' : '🌙'}</button>;
};

const TitleBar: React.FC = () => {
  return (
    <div className="title-bar">
      <h1>Man In The Middle Proxy</h1>
      <ThemeButton />
    </div>
  );
};

export default TitleBar;
