import { marks } from './lib/marks';
marks.module = performance.now();
import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import QuickWindow from './components/QuickWindow.svelte';

const quick = new URLSearchParams(location.search).get('view') === 'quick';
if (quick) document.body.classList.add('quick');

const app = mount(quick ? QuickWindow : App, { target: document.getElementById('app')! });
marks.mounted = performance.now();
export default app;
