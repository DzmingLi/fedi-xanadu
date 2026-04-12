import App from './App.svelte';
import { mount } from 'svelte';
import { checkOAuthSession } from './lib/auth.svelte';

// Check for OAuth cookie session (AT Protocol users returning from PDS auth)
checkOAuthSession();

const app = mount(App, { target: document.getElementById('app')! });

export default app;
