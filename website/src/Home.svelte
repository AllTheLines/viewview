<script lang="ts">
  import {
    DraftingCompass,
    Heart,
    Info,
    Monitor,
    Settings,
    TrophyIcon,
  } from '@lucide/svelte';
  import {
    LngLat,
    Map as MapLibre,
    NavigationControl,
    type StyleSpecification,
  } from 'maplibre-gl';
  import { onMount } from 'svelte';
  import { navigate } from 'svelte5-router';
  import ClickEffect, { initClickEffect } from './ClickEffect.svelte';
  import CollapsableModal from './components/CollapsableModal.svelte';
  import LayerToggle from './components/LayerToggle.svelte';
  import { HeatmapLayer } from './HeatmapLayer.ts';
  import heatmap_layer from './images/heatmap_layer.png';
  import mountain_peak from './images/mountain_peak.png';
  import vector_layer from './images/vector_layer.png';
  import Layout from './Layout.svelte';
  import map_vector from './map_vector.styles.json';
  import { render, setupLongestLines } from './renderLongestLine.ts';
  import Slider from './Slider.svelte';
  import { state } from './state.svelte.ts';
  import TopLines from './TopLines.svelte';
  import { lonLatRound } from './utils.ts';
  import {
    findLongestLineInBoundsBruteForce,
    findLongestLineInBoundsFromGrid,
  } from './worldLines.ts';

  let { longest } = $props();
  const minZoom = 1.6;
  const maxZoom = 15;
  const startingZoom = 2.0;
  const startingCentre = new LngLat(-5.0, 25.0);

  function addHeatmapLayer() {
    // 'mountain_peaks' is used here to mean, mountain peaks and every other layer after it.
    // This allows the heatmap to always appear below everything else.
    state.map?.addLayer(HeatmapLayer, 'mountain_peaks');
  }

  // Custom map bounds that allows hiding most of Antartica, whilst still allowing infinite horizontal
  // scroll.
  //
  // For an official fix, follow: https://github.com/maplibre/maplibre-gl-js/issues/6148
  function transformConstrain(lngLat: LngLat, zoom: number) {
    const latitudeToMercatorY = (latitude: number) => {
      return (
        0.5 -
        (0.25 *
          Math.log(
            (1 + Math.sin((latitude * Math.PI) / 180)) /
              (1 - Math.sin((latitude * Math.PI) / 180)),
          )) /
          Math.PI
      );
    };

    const mercatorYToLatitude = (mercatorY: number) => {
      return (
        (360 / Math.PI) * Math.atan(Math.exp((0.5 - mercatorY) * 2 * Math.PI)) -
        90
      );
    };

    const viewportHeight = state.map?.getContainer().clientHeight;
    if (viewportHeight === undefined) {
      return {
        center: startingCentre,
        zoom: startingZoom,
      };
    }

    const upperLatitudeBound = 85;
    const lowerLatitudeBound = -75;

    const worldSize = 512 * 2 ** zoom;
    const mercatorYOffset = viewportHeight / 2 / worldSize;

    const maxMercatorY = latitudeToMercatorY(upperLatitudeBound);
    const maxLatitude = mercatorYToLatitude(maxMercatorY + mercatorYOffset);
    const minMercatorY = latitudeToMercatorY(lowerLatitudeBound);
    const minLatitude = mercatorYToLatitude(minMercatorY - mercatorYOffset);

    const latitude = Math.max(minLatitude, Math.min(maxLatitude, lngLat.lat));

    return {
      center: new LngLat(lngLat.lng, latitude),
      zoom: Math.max(minZoom, Math.min(maxZoom, zoom)),
    };
  }

  async function updateTopLongestLines() {
    const bounds = state.map?.getBounds();
    if (bounds === undefined) {
      return;
    }

    state.longestLineInViewport = await findLongestLineInBoundsFromGrid(bounds);
  }

  onMount(() => {
    state.map = new MapLibre({
      container: 'map',
      zoom: startingZoom,
      center: startingCentre,
      style: map_vector as StyleSpecification,
      transformConstrain,
    });
    state.map.addControl(
      new NavigationControl({
        visualizePitch: true,
        visualizeRoll: true,
        showZoom: true,
      }),
      'bottom-right',
    );

    state.map.on('load', async () => {
      initClickEffect();
      if (longest === '') {
        addHeatmapLayer();
      }
      setupLongestLines(longest);
      await updateTopLongestLines();
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

      if (state.map?.getLayer(HeatmapLayer.id) === undefined) {
        addHeatmapLayer();
      }

      await updateTopLongestLines();
    });
  });
</script>

<Layout>
	<ClickEffect />
	<div id="info">
		<CollapsableModal collapsedIcon={Info} isOpen={!state.isFirstInteraction}>
			<h2>All The Views In The World</h2>
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

			<p>
				Click the <span class="unclickable_icon"><TrophyIcon /></span> icon below
				to see our curated list of the planet's longest lines of sight
			</p>

			<p>
				See <a href="https://alltheviews.world">alltheviews.world</a> for more details.
			</p>
		</CollapsableModal>

		<CollapsableModal collapsedIcon={TrophyIcon} isOpen={false}>
			<h2>Top Lines Of Sight</h2>
			<TopLines />
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

		{#if state.longestLine}
			<CollapsableModal collapsedIcon={DraftingCompass}>
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
			</CollapsableModal>
		{/if}

		<CollapsableModal collapsedIcon={Settings} isOpen={false}>
			<h2>Heatmap Settings</h2>
			<Slider setting={"contrast"} />
			<Slider setting={"intensity"} />
		</CollapsableModal>

		<CollapsableModal collapsedIcon={Heart} isOpen={false}>
			<h2>Acknowledgments</h2>
			<p>
				This project was made by <a href="https://tombh.co.uk">Tom</a> and
				<a href="https://ryan.berge.rs/">Ryan</a>.
			</p>

			<p>
				The raw data comes from NASA's
				<a href="https://www.earthdata.nasa.gov/data/instruments/srtm">SRTM</a>
				mission. We used the cleaned version by
				<a href="http://www.viewfinderpanoramas.org/">Jonathon de Ferranti</a>.
			</p>

			<p>
				The vector map data is hosted by Zsolt Ero's <a
					href="https://openfreemap.org/">openfreemap.org</a
				> project.
			</p>
		</CollapsableModal>
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

	.unclickable_icon {
		display: inline-block;
		scale: 0.7;
		translate: 0px 6px;
	}

	#load_longest_line_in_viewport_button {
		cursor: pointer;
	}
</style>
