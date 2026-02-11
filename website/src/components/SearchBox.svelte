<script lang="ts">
  import { onMount } from 'svelte';
  import { Search } from '@lucide/svelte';
  import { MapboxSearchBox } from '@mapbox/search-js-web';
  import { state } from '../state.svelte.ts';
  import { disablePointer } from '../utils.ts';

  onMount(() => {
    const searchBox = new MapboxSearchBox();
    searchBox.accessToken =
      'pk.eyJ1IjoidG9tYmgiLCJhIjoiY2p4cWlqNnY1MDFhZDNscXc5YXJpcTJzciJ9.7gGR5t8KEAY0ZoXfTVBcng';
    searchBox.options = {
      types: 'poi,place,country',
      poi_category: 'mountain,natural_feature',
    };
    searchBox.addEventListener('retrieve', (e) => {
      const feature = e.detail;
      const coordinates = feature.features[0]?.geometry.coordinates;
      state.map?.flyTo({
        center: [coordinates[0], coordinates[1]],
        zoom: 11,
      });
      state.isFlying = true;

      disablePointer();
    });

    /* #search-widget is hidden (display: none) on mobile until the user expands search;
    we still mount here so the box is ready. Guard in case the node isn't available yet. */
    const container = document.querySelector('#search-widget');
    if (container) {
      // @ts-expect-error: MapboxSearchBox is a custom element
      container.appendChild(searchBox);
    }
  });
</script>

<div id="search-box" class:is-open={state.isSearchOpen}>
	<div id="search-widget"></div>
	<button
		id="search-toggle"
		type="button"
		on:click={() => {
			state.isSearchOpen = true;
			state.isInfoOpen = false;
		}}
	>
		<Search size={18} />
	</button>
</div>

<style>
	#search-box {
		position: absolute;
		width: 300px;
		margin-left: 17px;
		margin-top: 17px;
	}

	#search-widget {
		width: 100%;
	}

	#search-toggle {
		all: unset;
		cursor: pointer;
		display: none;
	}

	/* Mobile: search behaves like a compact icon button that expands into the full input */
	@media (max-width: 700px) { /* maybe 730 waiting on tom */
		#search-box {
			width: 2.75rem;
			height: 2.5rem;
			display: flex;
			align-items: center;
			justify-content: center;
			overflow: hidden;
			background-color: white;
			border-radius: 3px;
			z-index: 2;
		}

		#search-toggle {
			display: flex;
			width: 1rem;
			height: 2.5rem;
			align-items: center;
			justify-content: center;
			color: #141f41;
		}

		/* collapsed: only icon button visible */
		#search-widget {
			display: none;
		}

		/* expanded: show full widget, hide icon, grow container */
		#search-box.is-open {
			width: min(18rem, 80vw);
			height: auto;
			overflow: visible;
			background-color: transparent;
			border-radius: 0;
		}

		#search-box.is-open #search-widget {
			display: block;
		}

		#search-box.is-open #search-toggle {
			display: none;
		}
	}
</style>

