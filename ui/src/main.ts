import { marks } from './lib/marks';
marks.module = performance.now();
import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';

const app = mount(App, { target: document.getElementById('app')! });
marks.mounted = performance.now();
export default app;
