<script lang="ts">
  import { type HeatmapConfig, state as sharedState } from './state.svelte.ts';

  let { setting }: { setting: keyof HeatmapConfig } = $props();
  let isDragging = $state(false);
  let value = $state(getConfig(setting));

  function updateConfig(setting: keyof HeatmapConfig, value: number) {
    value = 1 - value;

    switch (setting) {
      case 'contrast':
        sharedState.heatmapConfig.contrast = value;
        break;
      case 'intensity':
        sharedState.heatmapConfig.intensity = value;
        break;
    }
  }

  function getConfig(setting: keyof HeatmapConfig) {
    switch (setting) {
      case 'contrast':
        return 1 - sharedState.heatmapConfig.contrast;
      case 'intensity':
        return 1 - sharedState.heatmapConfig.intensity;
    }
  }

  // biome-ignore lint/correctness/noUnusedVariables: used in template
  function capitalise(string: string) {
    return string.charAt(0).toUpperCase() + string.slice(1);
  }

  // biome-ignore lint/correctness/noUnusedVariables: used in template
  function sliderAction(node: HTMLDivElement) {
    const updateValue = (clientX: number) => {
      const rectangle = node.getBoundingClientRect();
      const percentage = (clientX - rectangle.left) / rectangle.width;
      value = Math.max(0, Math.min(1, percentage));
      updateConfig(setting, value);
      sharedState.map?.redraw();
    };

    const handlePointerDown = (e: PointerEvent) => {
      isDragging = true;
      updateValue(e.clientX);
      window.addEventListener('pointermove', handlePointerMove);
      window.addEventListener('pointerup', handlePointerUp);
    };

    const handlePointerMove = (e: PointerEvent) => {
      if (isDragging) updateValue(e.clientX);
    };

    const handlePointerUp = () => {
      isDragging = false;
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    node.addEventListener('pointerdown', handlePointerDown);

    return {
      destroy() {
        node.removeEventListener('pointerdown', handlePointerDown);
      },
    };
  }
</script>

<div class="container">
	<p>{capitalise(setting)}: {value.toFixed(2)}</p>

	<div class="track" use:sliderAction>
		<div
			class="thumb"
			style:left="{value * 100}%"
			class:active={isDragging}
		></div>
	</div>
</div>

<style>
	.track {
		position: relative;
		width: 100%;
		min-width: 300px;
		height: 10px;
		background: #b8bdd6;
		border-radius: 5px;
		cursor: pointer;
		touch-action: none; /* Prevents scrolling while dragging */
	}

	.thumb {
		position: absolute;
		top: 50%;
		width: 20px;
		height: 20px;
		background: #fd6612;
		border-radius: 50%;
		transform: translate(-50%, -50%);
		transition: transform 0.1s ease;
	}

	.thumb.active {
		transform: translate(-50%, -50%) scale(1.1);
	}
</style>
