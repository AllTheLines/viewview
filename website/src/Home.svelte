<script lang="ts">
  import { DraftingCompass, Info } from '@lucide/svelte';
  import { Map as MapLibre, type StyleSpecification } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import CollapsableModal from './components/CollapsableModal.svelte';
  import LayerToggle from './components/LayerToggle.svelte';
  import { HeatmapLayer } from './HeatmapLayer.ts';
  import heatmap_layer from './images/heatmap_layer.png';
  import mountain_peak from './images/mountain_peak.png';
  import vector_layer from './images/vector_layer.png';
  import Layout from './Layout.svelte';
  import map_vector from './map_vector.styles.json';
  import { setup } from './renderLongestLine.ts';
  import { state } from './state.svelte.ts';
  import { lonLatRound } from './utils.ts';

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      zoom: 4.0,
      center: [11.218344, 50.025044],
      style: map_vector as StyleSpecification,
    });

    state.map?.on('load', () => {
      // 'mountain_peaks' is used here to mean, mountain peaks and every other layer after it.
      state.map?.addLayer(HeatmapLayer, 'mountain_peaks');
      setup();
    });

    state.map?.on('movestart', () => {
      if (!state.isFirstInteraction) {
        state.isFirstInteraction = true;
      }
    });
  });
</script>

<Layout>
	<div id="info">
		<CollapsableModal collapsedIcon={Info} isOpen={!state.isFirstInteraction}>
			<h1>All The Views</h1>
			<p>We've calculated all the views on the planet.</p>
			<p>
				Click on any point to show the longest line of sight at that location
			</p>
			<p>
				The lines are the theoretical ideals. They rely on perfect weather
				conditions and favourable refraction.
			</p>
			<p>
				Heatmap colours: the brighter the more and further you can see. The
				darker the less you can see.
			</p>
		</CollapsableModal>

		{#if state.longestLine}
			<CollapsableModal collapsedIcon={DraftingCompass}>
				<h2>Longest Line</h2>
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
			</CollapsableModal>
		{/if}
	</div>

	<div id="layout_toggles">
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
			callback={(isToggled) => {
				const layers = state.map?.getStyle().layers || [];
				for (const layer of layers) {
					if (layer.id == "mountain_peaks") continue;
					if (layer.id == "background") continue;
					if (layer.id == "longest-line-fill") continue;
					if (state.map?.getLayer(layer.id))
						state.map?.setLayoutProperty(
							layer.id,
							"visibility",
							isToggled ? "visible" : "none",
						);
				}
			}}
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
		/>
	</div>
</Layout>

<style>
	#info {
		position: fixed;
		top: 1em;
		right: 1em;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		width: max-content;
		justify-content: space-between;
		gap: 1em;
		min-width: 0;
		max-width: 400px;
	}

	#details {
		font-family: monospace;
		flex: 0 0 auto;
	}

	#layout_toggles {
		position: fixed;
		bottom: 1em;
		left: 1em;
		display: flex;
		flex-direction: row;
		justify-content: space-between;
		gap: 1em;
	}
</style>
