import { useEffect, useState } from 'react';

export const useThemeProvider = () => {
  const [isDark, setIsDark] = useState(false);

  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

    setIsDark(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches);

    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  useEffect(() => {
    // TODO: dark theme가 지원 되면 그때 주석 해제 @ohah
    // document.body.setAttribute('data-theme', isDark ? 'dark' : 'light');
    document.body.setAttribute('data-theme', 'light');
  }, [isDark]);
};
