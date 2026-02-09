<script lang="ts">
  import { LngLat } from 'maplibre-gl';
  import { navigate } from 'svelte5-router';
  import { render } from './renderLongestLine';
  import { LongestLineH3 } from './worldLines';

  // biome-ignore lint/correctness/noUnusedVariables: used in template
  const topLines: LongestLineH3[] = [
    new LongestLineH3(
      new LngLat(78.76539611816406, 36.31400680541992),
      530.8 * 1000,
      '🇨🇳🇰🇬 Hindu Kush/Pik Dankova',
    ),
    new LongestLineH3(
      new LngLat(-75.72223663330078, 6.75514030456543),
      504.1 * 1000,
      '🇨🇴 Antioquia/Pico Cristobal',
    ),
    new LongestLineH3(
      new LngLat(42.43996810913086, 43.34897232055664),
      483.1 * 1000,
      '🇷🇺🇹🇷 Mt. Elbrus/Pontic Mountains',
    ),
    new LongestLineH3(
      new LngLat(97.25302124023438, 28.01169204711914),
      480.4 * 1000,
      '🇮🇳 Patkai Hills/Gori Chen',
    ),
    new LongestLineH3(
      new LngLat(-91.90559387207031, 15.041804313659668),
      477.9 * 1000,
      '🇬🇹🇲🇽 Tajumulco/Nube Flane',
    ),
    new LongestLineH3(
      new LngLat(69.3552017211914, 39.26898956298828),
      475.3 * 1000,
      '🇹🇯🇦🇫 Ravosang Pass/Ghowr',
    ),
    new LongestLineH3(
      new LngLat(71.8376235961914, 36.432369232177734),
      470.1 * 1000,
      '🇵🇰🇺🇿 Noshaq/Aghbai Kalon',
    ),
    new LongestLineH3(
      new LngLat(162.6785430908203, -78.0618667602539),
      463.8 * 1000,
      '🇦🇶 Antarctica',
    ),
    new LongestLineH3(
      new LngLat(46.122562408447266, 42.21756362915039),
      462.9 * 1000,
      '🇷🇺🇮🇷 Balakuri/Sabalan',
    ),
    new LongestLineH3(
      new LngLat(146.864501953125, -6.220884323120117),
      451.6 * 1000,
      '🇵🇬 Kotewon/Suckling',
    ),
  ];
</script>

<ol>
	{#each topLines as line}
		<li>
			<a
				href={line.toURL()}
				onclick={(event) => {
					event.preventDefault();
					if (line !== undefined) {
						const url = line.toURL();
						render(line.coordinate);
						navigate(url);
					}
				}}>{line.toDistance()}</a
			>
			{line.description}
		</li>
	{/each}
</ol>
