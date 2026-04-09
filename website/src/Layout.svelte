<script lang="ts">
  import 'maplibre-gl/dist/maplibre-gl.css';
  import 'accessible-nprogress/src/styles.css';
  import SearchBox from './components/SearchBox.svelte';
  import { state } from './state.svelte.ts';
</script>

{#if state.config.project === "world"}
	<SearchBox />
{/if}

<main>
	<slot />
</main>
