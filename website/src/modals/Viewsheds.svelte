<script lang="ts">
  import { Eye, EyeClosed, LockOpen, Rainbow, Trash } from '@lucide/svelte';
  import CollapsableModal from '../components/CollapsableModal.svelte';
  import {
    desaturateAllViewsheds,
    highlightViewshed,
    removeViewshed,
    renderAllViewsheds,
    updateViewshedColour,
  } from '../lib/renderViewsheds';
  import { state } from '../state.svelte';

  let _colourInput: HTMLInputElement;
</script>

<CollapsableModal collapsedIcon={Rainbow} isOpen={true}>
	<h2>Viewsheds</h2>
	{#if state.viewsheds.length == 0}
		Click any point to render its viewshed
	{/if}

	{#if state.viewsheds.length > 0}
		<div id="viewsheds">
			{#each state.viewsheds as viewshed}
				<div
					role="button"
					tabindex="0"
					class="viewshed locked-{viewshed.isLocked ? 'true' : 'false'}"
					onmouseenter={() => {
						if (viewshed.isVisible) {
							highlightViewshed(viewshed);
						}
					}}
					onmouseleave={() => {
						desaturateAllViewsheds();
					}}
				>
					<button
						class="visible visible-{viewshed.isVisible ? 'true' : 'false'}"
						onclick={() => {
							viewshed.isVisible = !viewshed.isVisible;
							state.viewsheds = state.viewsheds;
							renderAllViewsheds();
						}}
					>
						{#if viewshed.isVisible}
							<Eye />
						{:else}
							<EyeClosed />
						{/if}
					</button>

					<button
						class="lock"
						onclick={() => {
							if (viewshed.isLocked) {
								removeViewshed(viewshed);
							} else {
								viewshed.isLocked = !viewshed.isLocked;
								state.viewsheds = state.viewsheds;
								renderAllViewsheds();
							}
						}}
					>
						{#if viewshed.isLocked}
							<Trash />
						{:else}
							<LockOpen />
						{/if}
					</button>

					<button
						class="colour-button"
						title="Colour Picker"
						onclick={() => _colourInput.click()}
					>
						<input
							bind:this={_colourInput}
							type="color"
							bind:value={viewshed.colour}
							oninput={() => {
								state.viewsheds = state.viewsheds;
								updateViewshedColour(viewshed);
							}}
							style="display: none;"
						/>
						<span
							class="colour-indicator"
							style="background-color: {viewshed.colour};"
						></span>
					</button>

					{viewshed.centre.lat.toFixed(5)},{viewshed.centre.lng.toFixed(5)}
				</div>
			{/each}
		</div>
	{/if}
</CollapsableModal>

<style>
	button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		height: 2em;
		width: 2em;
	}

	.viewshed {
		display: flex;
		flex-direction: row;
		align-items: center;
		height: 1em;
		gap: 0.5em;
		padding: 1em;
		border-radius: 3px;

		&:hover {
			background-color: #fed7c2;
		}

		&.locked-false {
			opacity: 0.5;
		}
	}

	.colour-indicator {
		display: inline-block;
		width: 1.25em;
		height: 1.25em;
		border-radius: 2px;
	}
</style>
