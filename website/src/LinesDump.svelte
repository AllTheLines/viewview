<script lang="ts">
  import { navigate } from 'svelte5-router';
  import { render } from './renderLongestLine';
  import { state } from './state.svelte.ts';
  import { loadH3Lines } from './worldLines';
  import { onMount } from 'svelte';

  onMount(async () => {
    if (state.worldLongestLines === undefined) {
      await loadH3Lines();
    }
  });
</script>

<main>
	<ol>
		{#each state.worldLongestLines?.slice(0, 1000) as line}
			<li>
				<a
					href={line.toURL()}
					onclick={(event) => {
						event.preventDefault();
						if (line !== undefined) {
							const url = line.toURL();
							render(line.coordinate);
							navigate(url);
						}
					}}>{line.toDistance()}</a
				>
			</li>
		{/each}
	</ol>
</main>

<style>
	main {
		margin: 2em;
	}
</style>
