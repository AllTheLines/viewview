<script lang="ts">
  import { Settings } from '@lucide/svelte';

  import CollapsableModal from '../components/CollapsableModal.svelte';
  import { changeRunSource } from '../lib/renderLongestLine';
  import Slider from '../Slider.svelte';
  import { state } from '../state.svelte';
</script>

<CollapsableModal
	collapsedIcon={Settings}
	isOpen={state.isSettingsOpen}
	onOpen={() => {
		state.isSettingsOpen = true;
	}}
	onClose={() => {
		state.isSettingsOpen = false;
	}}
>
	<h2>Heatmap Settings</h2>
	<Slider setting={"contrast"} />
	<Slider setting={"intensity"} />

	<h2>Data source</h2>
	<form>
		<fieldset>
			<div>
				<input
					type="radio"
					name="refraction-value"
					id="refraction-013"
					bind:group={state.heatmapConfig.longestLinesRun}
					value="ryan-fullworld-raw"
					onchange={async () => await changeRunSource()}
				/>
				<label for="refraction-013">January 20th 2026. Refraction: 0.13</label>
			</div>

			<div>
				<input
					type="radio"
					name="refraction-value"
					id="refraction-023"
					bind:group={state.heatmapConfig.longestLinesRun}
					value="ryan-world2-run-refraction23"
					onchange={async () => await changeRunSource()}
				/>
				<label for="refraction-023">June 30th 2026. Refraction: 0.23</label>
			</div>
		</fieldset>
	</form>
</CollapsableModal>
