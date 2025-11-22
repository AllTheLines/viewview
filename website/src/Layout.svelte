<script lang="ts">
  import { Map as MapLibre, type StyleSpecification } from 'maplibre-gl';
  import { onDestroy, onMount } from 'svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'accessible-nprogress/src/styles.css';
  import map_vector from './map_vector.styles.json';
  import Sidebar from './Sidebar.svelte';
  import { state } from './state.svelte.ts';

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      zoom: 5.5,
      center: [-3, 54],
      style: map_vector as StyleSpecification,
    });
  });

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
