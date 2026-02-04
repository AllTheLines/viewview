<script lang="ts" context="module">
  export function initClickEffect() {
    const container = document
      .getElementsByClassName('maplibregl-interactive')
      .item(0) as HTMLElement;
    const click_effect = document.getElementById('click_effect') as HTMLElement;

    if (!container || !click_effect) {
      return;
    }

    container.addEventListener('click', (event: MouseEvent) => {
      const x = event.clientX;
      const y = event.clientY;

      click_effect.style.left = `${x}px`;
      click_effect.style.top = `${y}px`;
      click_effect.style.transform = 'scale(3)';
      click_effect.style.opacity = '1';

      setTimeout(() => {
        click_effect.style.transform = 'scale(0)';
        click_effect.style.opacity = '0';
      }, 300);
    });
  }
</script>

<div id="click_effect"></div>

<style>
	#click_effect {
		width: 8px;
		height: 8px;
		position: absolute;
		border-radius: 50%;
		background: var(--longest-line);
		transform: scale(0);
		pointer-events: none; /* Ignore mouse events */
		transition:
			transform 0.3s ease,
			opacity 0.3s ease;
		opacity: 0;
	}
</style>
