<script lang="ts">
  import { onDestroy } from 'svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'accessible-nprogress/src/styles.css';
  import Sidebar from './Sidebar.svelte';
  import { state } from './state.svelte.ts';

  onDestroy(() => {
    state.map?.remove();
  });
</script>

<svelte:head>
	<title>All The Views</title>
</svelte:head>

<div id="map"></div>

<Sidebar />

<main>
	<slot />
</main>

<style>
	#map {
		position: absolute;
		height: 100%;
		width: 100%;
	}
</style>
