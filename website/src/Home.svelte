<script lang="ts">
  import { onMount } from 'svelte';
  import { HeatmapLayer } from './HeatmapLayer.ts';
  import Layout from './Layout.svelte';
  import { state } from './state.svelte.ts';

  onMount(() => {
    state.map?.on('load', () => {
      state.map?.addLayer(HeatmapLayer);
    });
  });
</script>
 
<Layout></Layout>
