<script lang="ts">
  import { Minimize2 } from '@lucide/svelte';
  import type { Component } from 'svelte';

  export let collapsedIcon: Component;
  const __buttonSize = 18;
  let __isOpen = true;
</script>

<div class="collapseable_modal {__isOpen ? '' : 'modal__collapsed'}">
	<div>
		{#if __isOpen}
			<button class="modal__close" on:click={() => (__isOpen = false)}>
				<Minimize2 size={__buttonSize} />
			</button>
			<slot />
		{:else}
			<button class="modal__open" on:click={() => (__isOpen = true)}>
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
				color: red;
			}
		}

		.modal__open {
			&:hover {
				color: green;
			}
		}
	}
</style>
