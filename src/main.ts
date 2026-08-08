import { mount } from 'svelte';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import '@fontsource/ibm-plex-mono/600.css';
import '@fontsource/ibm-plex-mono/700.css';
import './styles.css';
import './lib/theme/theme.svelte';
import App from './App.svelte';

const app = mount(App, { target: document.getElementById('app')! });

export default app;
