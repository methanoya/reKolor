// The app's entry point, loaded by `index.html`. Svelte compiles each `.svelte` component into
// plain JavaScript; `mount` creates the root component (`App`) inside the `<div id="app">` of the
// page. Importing the CSS file makes Vite add it to the page.
import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';

// `!` tells TypeScript the element exists (`getElementById` may return `null`).
mount(App, { target: document.getElementById('app')! });
