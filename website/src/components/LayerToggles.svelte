<script lang="ts">
  import { HeatmapLayer } from '../HeatmapLayer';
  import heatmap_layer from '../images/heatmap_layer.png';
  import mountain_peak from '../images/mountain_peak.png';
  import vector_layer from '../images/vector_layer.png';
  import { setVectorVisibility } from '../lib/utils';
  import { state } from '../state.svelte';
  import LayerToggle from './LayerToggle.svelte';
</script>

<div id="layer_toggles">
	<LayerToggle
		image={heatmap_layer}
		callback={(isToggled) => {
			state.map?.setLayoutProperty(
				HeatmapLayer.id,
				"visibility",
				isToggled ? "visible" : "none",
			);
		}}
	/>
	<LayerToggle
		image={vector_layer}
		callback={(isToggled) => setVectorVisibility(state, isToggled)}
		isToggled={state.config.project === "world"}
	/>
	<LayerToggle
		image={mountain_peak}
		callback={(isToggled) => {
			state.map?.setLayoutProperty(
				"mountain_peaks",
				"visibility",
				isToggled ? "visible" : "none",
			);
		}}
		isToggled={state.config.project === "world"}
	/>
</div>

<style lang="scss">
	@use "../styles/variables.scss" as *;

	#layer_toggles {
		position: fixed;
		bottom: 1em;
		left: 1em;
		display: flex;
		flex-direction: row;
		justify-content: space-between;
		gap: 1em;
	}

	/* Mobile-only layout tweaks */
	@media (max-width: $mobile-break) {
		#layer_toggles {
			left: 0.5rem;
			bottom: 0.5rem;
			gap: 0.4rem;
			flex-direction: column;
		}
	}
</style>
