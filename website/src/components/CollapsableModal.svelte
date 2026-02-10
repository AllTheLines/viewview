<script lang="ts">
  import { Minimize2 } from '@lucide/svelte';
  import type { Component } from 'svelte';
  import { createEventDispatcher } from 'svelte';

  export let collapsedIcon: Component;
  export let isOpen = true;
  const __buttonSize = 18;
  const _dispatch = createEventDispatcher();
</script>

<div class="collapseable_modal {isOpen ? '' : 'modal__collapsed'}">
	<div>
		{#if isOpen}
			<button
				class="modal__close"
				on:click={() => {
					isOpen = false;
					_dispatch('close');
				}}
			>
				<Minimize2 size={__buttonSize} />
			</button>
			<slot />
		{:else}
			<button
				class="modal__open"
				on:click={() => {
					isOpen = true;
					_dispatch('open');
				}}
			>
				<svelte:component this={collapsedIcon} size={__buttonSize} />
			</button>
		{/if}
	</div>
</div>

<style>
	.modal__collapsed {
		align-self: flex-end;
	}

	.collapseable_modal {
		background-color: white;
		border-radius: 3px;

		> div {
			padding: 1em;
			position: relative;
		}

		button {
			all: unset;
			cursor: pointer;
		}

		.modal__close {
			position: absolute;
			top: 1em;
			right: 1em;
			&:hover {
				color: var(--primary-colour);
			}
		}

		.modal__open {
			&:hover {
				color: var(--primary-colour);
			}
		}
	}
</style>
