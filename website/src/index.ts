import { mount } from 'svelte';
import App from './App.svelte';
import './styles/index.scss';

const app = mount(App, {
  target: document.getElementById('root') as HTMLElement,
});

export default app;
