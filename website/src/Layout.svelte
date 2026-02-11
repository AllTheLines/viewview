<script lang="ts">
  import { onDestroy } from 'svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'accessible-nprogress/src/styles.css';
  import { state } from './state.svelte.ts';
  import SearchBox from './components/SearchBox.svelte';

  onDestroy(() => {
    state.map?.remove();
  });
</script>

<div id="map"></div>

<SearchBox />

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
