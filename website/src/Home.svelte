<script lang="ts">
  import { DraftingCompass, Info, TrophyIcon } from '@lucide/svelte';
  import { Map as MapLibre, type StyleSpecification } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import { navigate } from 'svelte5-router';
  import CollapsableModal from './components/CollapsableModal.svelte';
  import LayerToggle from './components/LayerToggle.svelte';
  import { HeatmapLayer } from './HeatmapLayer.ts';
  import heatmap_layer from './images/heatmap_layer.png';
  import mountain_peak from './images/mountain_peak.png';
  import vector_layer from './images/vector_layer.png';
  import Layout from './Layout.svelte';
  import map_vector from './map_vector.styles.json';
  import { render, setupLongestLines } from './renderLongestLine.ts';
  import { state } from './state.svelte.ts';
  import { lonLatRound } from './utils.ts';
  import {
    findLongestLineInBoundsBruteForce,
    findLongestLineInBoundsFromGrid,
  } from './worldLines.ts';

  let { longest } = $props();

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      zoom: 2.5,
      center: [-30.0, 34.11871197394943],
      style: map_vector as StyleSpecification,
    });

    state.map.on('load', () => {
      // 'mountain_peaks' is used here to mean, mountain peaks and every other layer after it.
      // This allows the heatmap to always appear below everything else.
      state.map?.addLayer(HeatmapLayer, 'mountain_peaks');
      setupLongestLines(longest);
    });

    state.map.on('movestart', () => {
      if (!state.isFirstInteraction) {
        state.isFirstInteraction = true;
      }
    });

    state.map?.on('moveend', async () => {
      if (state.map === undefined) {
        return;
      }

      const bounds = state.map?.getBounds();
      if (bounds !== undefined) {
        let longest = await findLongestLineInBoundsFromGrid(bounds);
        if (longest === undefined) {
          state.longestLineInViewport = undefined;
          longest = await findLongestLineInBoundsBruteForce(bounds);
        }
        state.longestLineInViewport = longest;
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

		<CollapsableModal collapsedIcon={TrophyIcon} isOpen={false}>
			<h2>Longest Lines</h2>
			<ol>
				{#each state.worldLongestLines?.slice(0, 10) as line}
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
					}}>In viewport ({state.longestLineInViewport?.toDistance()})</a
				>
			{:else}
				In viewport (loading...)
			{/if}
		</CollapsableModal>

		{#if state.longestLine}
			<CollapsableModal collapsedIcon={DraftingCompass}>
				<h2>Current line of sight</h2>
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
