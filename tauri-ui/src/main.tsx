import React from 'react';
import { createRoot } from 'react-dom/client';
import App from './components/App';
import './main.css';
import '../styles.css';
import './stores/session';

const container = document.getElementById('root');

if (container) {
  const root = createRoot(container);
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
} else {
  console.error('Failed to find the root element');
}
