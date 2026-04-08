<script lang="ts">
  import { DraftingCompass } from '@lucide/svelte';
  import CollapsableModal from '../components/CollapsableModal.svelte';
  import { lonLatRound } from '../lib/utils';
  import { state } from '../state.svelte';
</script>

<CollapsableModal collapsedIcon={DraftingCompass}>
	{#if state.longestLine}
		<h2>Current Line Of Sight</h2>
		<div id="details">
			<div>
				Distance: {(state.longestLine.distance || 0) / 1000}km
			</div>
			<div>
				Bearing: {state.longestLine.angle}°
			</div>
			<div>
				From: {lonLatRound(state.longestLine.from)}
			</div>
			<div>
				To: {lonLatRound(state.longestLine.to)}
			</div>
		</div>
	{/if}
</CollapsableModal>

<style>
	#details {
		font-family: monospace;
		flex: 0 0 auto;
	}
</style>
