<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'accessible-nprogress/src/styles.css';
  import { MapboxSearchBox } from '@mapbox/search-js-web';
  import { state } from './state.svelte.ts';
  import { disablePointer } from './utils.ts';

  onMount(() => {
    const searchBox = new MapboxSearchBox();
    searchBox.accessToken =
      'pk.eyJ1IjoidG9tYmgiLCJhIjoiY2p4cWlqNnY1MDFhZDNscXc5YXJpcTJzciJ9.7gGR5t8KEAY0ZoXfTVBcng';
    searchBox.options = {
      types: 'poi,place,country',
      poi_category: 'mountain,natural_feature',
    };
    searchBox.addEventListener('retrieve', (e) => {
      const feature = e.detail;
      const coordinates = feature.features[0]?.geometry.coordinates;
      state.map?.flyTo({
        center: [coordinates[0], coordinates[1]],
        zoom: 11,
      });
      state.isFlying = true;

      disablePointer();
    });

    /* #search-widget is hidden (display: none) on mobile until the user expands search;
    we still mount here so the box is ready. Guard in case the node isn't available yet. */
    const container = document.querySelector('#search-widget');
    if (container) {
      // @ts-expect-error: MapboxSearchBox is a custom element
      container.appendChild(searchBox);
    }
  });

  onDestroy(() => {
    state.map?.remove();
  });
</script>

<div id="map"></div>

<div id="search-box"></div>

<main>
	<slot />
</main>

<style>
	#map {
		position: absolute;
		height: 100%;
		width: 100%;
	}

	#search-box {
		position: absolute;
		width: 300px;
		margin-left: 17px;
		margin-top: 17px;
	}
</style>
