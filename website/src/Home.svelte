<script lang="ts">
  import { Settings } from '@lucide/svelte';
  import {
    getRTLTextPluginStatus,
    Map as MapLibre,
    NavigationControl,
    type StyleSpecification,
    setRTLTextPlugin,
  } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import ClickEffect, { initClickEffect } from './ClickEffect.svelte';
  import CollapsableModal from './components/CollapsableModal.svelte';
  import LayerToggles from './components/LayerToggles.svelte';
  import { getConfig } from './config.ts';
  import { HeatmapLayer } from './HeatmapLayer.ts';
  import Layout from './Layout.svelte';
  import { transformConstrain } from './lib/mapConstrains.ts';
  import { setupLongestLines } from './lib/renderLongestLine.ts';
  import { setupViewsheds } from './lib/renderViewsheds.ts';
  import { enablePointer, setVectorVisibility } from './lib/utils.ts';
  import { findLongestLineInBoundsFromGrid } from './lib/worldLines.ts';
  import map_vector from './map_vector.styles.json';
  import Acknowledgements from './modals/Acknowledgements.svelte';
  import CurrentLine from './modals/CurrentLine.svelte';
  import TopLines from './modals/TopLines.svelte';
  import Viewsheds from './modals/Viewsheds.svelte';
  import Welcome from './modals/Welcome.svelte';
  import Slider from './Slider.svelte';
  import { state } from './state.svelte.ts';

  let { longest } = $props();
  const config = getConfig();

  function addHeatmapLayer() {
    if (state.map?.getLayer(HeatmapLayer.id) !== undefined) {
      return;
    }

    // 'mountain_peaks' is used here to mean, mountain peaks and every other layer after it.
    // This allows the heatmap to always appear below everything else.
    state.map?.addLayer(HeatmapLayer, 'mountain_peaks');
  }

  async function updateTopLongestLines() {
    const bounds = state.map?.getBounds();
    if (bounds === undefined) {
      return;
    }

    state.longestLineInViewport = await findLongestLineInBoundsFromGrid(bounds);
  }

  onMount(async () => {
    state.map = new MapLibre({
      container: 'map',
      zoom: config.map.startingZoom,
      center: config.map.startingCentre,
      style: map_vector as StyleSpecification,
      transformConstrain: transformConstrain(state),
    });

    state.map.addControl(
      new NavigationControl({
        visualizePitch: true,
        visualizeRoll: true,
        showZoom: true,
      }),
      'bottom-right',
    );

    if (
      typeof window !== 'undefined' &&
      getRTLTextPluginStatus() === 'unavailable'
    ) {
      // https://maplibre.org/maplibre-gl-js/docs/API/functions/setRTLTextPlugin/
      await setRTLTextPlugin(
        'https://unpkg.com/@mapbox/mapbox-gl-rtl-text@0.3.0/dist/mapbox-gl-rtl-text.js',
        true, // Lazy load the plugin only when text is in arabic
      );
    }

    state.map.on('load', async () => {
      initClickEffect();

      if (longest === '') {
        addHeatmapLayer();
      }

      if (state.config.project === 'world') {
        setupLongestLines(longest);
        setVectorVisibility(state, true);
        await updateTopLongestLines();
      }

      if (state.config.project === 'galiano') {
        setupViewsheds();
      }
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

      addHeatmapLayer();

      if (state.config.project === 'world') {
        await updateTopLongestLines();
      }
    });

    state.map?.on('moveend', async () => {
      if (state.isFlying) {
        enablePointer();
        state.isFlying = false;
      }
    });
  });
</script>

<div id="map"></div>

<Layout>
	<ClickEffect />

	<div id="info">
		{#if state.config.project === "galiano"}
			<Viewsheds />
		{/if}

		{#if state.config.project === "world"}
			<Welcome />
		{/if}

		{#if state.config.project === "world"}
			<TopLines />
		{/if}

		{#if state.longestLine}
			<CurrentLine />
		{/if}

		<CollapsableModal collapsedIcon={Settings} isOpen={false}>
			<h2>Heatmap Settings</h2>
			<Slider setting={"contrast"} />
			<Slider setting={"intensity"} />
		</CollapsableModal>

		{#if state.config.project === "world"}
			<Acknowledgements />
		{/if}
	</div>

	<LayerToggles />
</Layout>

<style lang="scss">
	@use "./styles/variables.scss" as *;

	#map {
		position: absolute;
		height: 100%;
		width: 100%;
	}

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

	/* Mobile-only layout tweaks */
	@media (max-width: $mobile-break) {
		/* slightly smaller collapsed modal icons on Home only */
		:global(#info .collapseable_modal .modal__open svg) {
			width: 1rem;
			height: 1rem;
		}

		/* align top of first info button roughly with search bar */
		#info {
			top: 1.08rem;
			right: 0.75rem;
			max-width: min(22rem, 80vw);
			gap: 0.75rem;
		}

		:global(#info .collapseable_modal > div) {
			padding: 0.75em;
		}
	}
</style>
