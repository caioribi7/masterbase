import { mount } from 'svelte';
import '@fontsource/tektur/800.css';
import '@fontsource/geologica/400.css';
import '@fontsource/geologica/500.css';
import '@fontsource/geologica/600.css';
import '@fontsource/martian-mono/400.css';
import './app.css';
import App from './App.svelte';
import { applyPrefs } from './prefs.js';

applyPrefs();

mount(App, { target: document.getElementById('app') });
