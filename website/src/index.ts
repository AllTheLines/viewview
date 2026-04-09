import { mount } from 'svelte';
import App from './App.svelte';
import './styles/index.scss';
import { state } from './state.svelte';

const root = document.getElementById('root') as HTMLElement;

// Because of MaplibreGL we have to manually handle some post PMR setup
root.innerHTML = '';

const app = mount(App, {
  target: root,
});

if (import.meta.webpackHot) {
  import.meta.webpackHot.accept();
  import.meta.webpackHot.dispose(() => {
    state.map?.remove();
  });
}

export default app;
