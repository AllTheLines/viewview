<script lang="ts">
  import { Monitor, TrophyIcon } from '@lucide/svelte';
  import { navigate } from 'svelte5-router';
  import CollapsableModal from '../components/CollapsableModal.svelte';
  import { render } from '../lib/renderLongestLine';
  import { findLongestLineInBoundsBruteForce } from '../lib/worldLines';
  import { state } from '../state.svelte';
  import TopLinesContent from './TopLinesContent.svelte';
</script>

<CollapsableModal collapsedIcon={TrophyIcon} isOpen={false}>
	<h2>Top Lines Of Sight</h2>
	<TopLinesContent />
	In current viewport<span class="unclickable_icon"><Monitor /></span>:
	{#if state.longestLineInViewport}
		<a
			href={state.longestLineInViewport?.toURL()}
			onclick={(event) => {
				event.preventDefault();
				if (state.longestLineInViewport !== undefined) {
					const url = state.longestLineInViewport?.toURL();
					render(state.longestLineInViewport.coordinate);
					navigate(url);
				}
			}}>{state.longestLineInViewport?.toDistance()}</a
		>
	{:else if state.bruteForceLoadingLine}
		loading...
	{:else}
		<button
			id="load_longest_line_in_viewport_button"
			onclick={async (event) => {
				event.preventDefault();
				const bounds = state.map?.getBounds();
				if (bounds === undefined) {
					return;
				}
				state.bruteForceLoadingLine = true;
				let longest = await findLongestLineInBoundsBruteForce(bounds);
				state.bruteForceLoadingLine = false;
				state.longestLineInViewport = longest;
			}}>load</button
		>
	{/if}
</CollapsableModal>

<style lang="scss">
	#load_longest_line_in_viewport_button {
		cursor: pointer;
	}
</style>
