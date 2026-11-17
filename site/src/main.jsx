import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource/tektur/600.css';
import '@fontsource/tektur/700.css';
import '@fontsource/geologica/400.css';
import '@fontsource/geologica/500.css';
import '@fontsource/geologica/600.css';
import '@fontsource/martian-mono/400.css';
import './styles.css';
import App from './App.jsx';

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
