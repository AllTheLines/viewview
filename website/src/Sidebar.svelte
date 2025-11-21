<script lang="ts">
  import { Menu, X } from '@lucide/svelte';
  import { slide } from 'svelte/transition';

  let __isOpen = false;
</script>

{#if __isOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="overlay" on:click={() => (__isOpen = false)}></div>
{/if}

{#if !__isOpen}
	<button class="toggle sidebar__closed" on:click={() => (__isOpen = true)}>
		<Menu />
	</button>
{:else}
	<nav class="sidebar sidebar__open" transition:slide={{ duration: 200 }}>
		<div>
			<button class="toggle" on:click={() => (__isOpen = false)}>
				<X />
			</button>
			<h2>All The Views</h2>
			<ul>
				<li>
					<a href="/">Home</a>
				</li>
				<li>
					<a href="/tile-packing-the-world">Tile Packing</a>
				</li>
			</ul>
		</div>
	</nav>
{/if}

{#if __isOpen}{/if}

<style>
	nav {
		margin: 1em;
	}

	ul,
	li {
		all: unset;
	}

	li {
		margin-left: 1em;
		margin-bottom: 0.5em;
		display: block;
	}

	button.toggle {
		position: absolute;
		top: 1rem;
		font-size: 2rem;
		background: none;
		border: none;
		cursor: pointer;
		z-index: 1000001;
	}

	button.sidebar__closed {
		left: 1rem;
		color: #fff;
	}

	.sidebar__open {
		button.toggle {
			right: 1rem;
		}
	}

	.overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.7);
		z-index: 1000;
	}

	nav > div {
		position: relative;
		padding: 2rem 1rem;
	}

	nav.sidebar {
		position: fixed;
		top: 0;
		left: 0;
		height: 100vh;
		width: 260px;
		background: #fff;
		border-right: 1px solid #ddd;
		box-shadow: 2px 0 6px rgba(0, 0, 0, 0.1);
		z-index: 1002;

		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	nav.sidebar a {
		text-decoration: none;
		color: #333;
		font-size: 1.1rem;
	}

	nav.sidebar a:hover {
		color: #000;
	}
</style>
