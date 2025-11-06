<script lang="ts">
  import { Popup } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import Layout from './Layout.svelte';
  import { state } from './state.svelte.ts';

  onMount(() => {
    state.map?.on('load', async () => {
      const result = await fetch('tiles.json');
      if (!result.ok) {
        throw new Error(`Failed to fetch tiles.json: ${result.status}`);
      }
      const geojson = await result.json();
      state.map?.addSource('my-geojson', {
        type: 'geojson',
        data: geojson,
      });
      state.map?.addLayer({
        id: 'fills',
        type: 'fill',
        source: 'my-geojson',
        paint: {
          'fill-outline-color': '#2b8cbe',
          'fill-color': '#7fb3d5',
          'fill-opacity': 0.35,
        },
      });
      state.map?.on('click', 'fills', (event) => {
        if (!state.map) {
          return;
        }
        const props = event.features?.[0].properties;
        let message = JSON.stringify(props, null, 2);
        message = message
          .replace('[', '[\n    ')
          .replace(']', '\n  ]')
          .replace(',', ',\n    ');
        new Popup()
          .setLngLat(event.lngLat)
          .setHTML(`<pre>${message}</pre>`)
          .addTo(state.map);
      });
      state.map?.on('mouseenter', 'fills', () => {
        if (!state.map) {
          return;
        }
        state.map.getCanvas().style.cursor = 'pointer';
      });
      state.map?.on('mouseleave', 'fills', () => {
        if (!state.map) {
          return;
        }
        state.map.getCanvas().style.cursor = '';
      });
    });
  });
</script>
  
<Layout></Layout>

