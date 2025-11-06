<script lang="ts">
  import { Map as MapLibre } from 'maplibre-gl';
  import { onDestroy, onMount } from 'svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { state } from './state.svelte.ts';

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      zoom: 1,
      center: [-3, 53],
      style: {
        version: 8,
        sources: {
          osm: {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution:
              '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
          },
        },
        layers: [
          {
            id: 'osm-layer',
            type: 'raster',
            source: 'osm',
            minzoom: 0,
            maxzoom: 14,
          },
        ],
      },
    });
  });

  onDestroy(() => {
    state.map?.remove();
  });
</script>
 
<div id="map"></div>
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
