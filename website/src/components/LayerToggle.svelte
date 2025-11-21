<script lang="ts">
  export let image: string;
  export let callback: (arg0: boolean) => void;
  let isToggled = true;

  function __toggle() {
    isToggled = !isToggled;
    callback(isToggled);
  }
</script>

<button
	class="layer_toggle layer_toggle__{isToggled ? 'on' : 'off'}"
	on:click={__toggle}
>
	<img src={image} alt="Layer toggle" />
</button>

<style>
	button {
		all: unset;
		cursor: pointer;
	}

	.layer_toggle {
		border-radius: 10px;
		background-color: #303030;
		width: 80px;
		height: 80px;
		img {
			width: 100%;
			height: 100%;
			border-radius: 10px;
		}
		&.layer_toggle__off {
			border: 2px solid #303030;
			img {
				opacity: 0.4;
				&:hover {
					opacity: 0.7;
				}
			}
		}
		&.layer_toggle__on {
			border: 2px solid green;
			border-color: green;
			img {
				opacity: 1;
				&:hover {
					opacity: 0.7;
				}
			}
		}
	}
</style>
