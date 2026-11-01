<script lang="ts">
  import { LngLat } from 'maplibre-gl';
  import { navigate } from 'svelte5-router';
  import { render } from '../lib/renderLongestLine';
  import { LongestLineH3 } from '../lib/worldLines';
  import { type RunName, state } from '../state.svelte';

  const activeRun = state.heatmapConfig.longestLinesRun;

  type Details = {
    runName: RunName;
    x: number;
    y: number;
    distance: number;
  };

  type Lines = {
    numbers: Details[];
    title: string;
  };

  function makeLine(lines: Lines) {
    const line = lines.numbers.find((line) => line.runName === activeRun);
    if (line === undefined) {
      throw Error;
    }
    return new LongestLineH3(
      new LngLat(line.x, line.y),
      line.distance * 1000,
      lines.title,
    );
  }

  // biome-ignore lint/correctness/noUnusedVariables: used in template
  const topLines: LongestLineH3[] = [
    makeLine({
      title: '🇨🇳🇰🇬 Kunlun Mountains/Pik Dankova',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 78.76539611816406,
          y: 36.31400680541992,
          distance: 530.8,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 77.60699462890625,
          y: 41.008506774902344,
          distance: 576.9,
        },
      ],
    }),
    makeLine({
      title: '🇨🇴 Cerro Lago/Pico Cristobal',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: -75.72223663330078,
          y: 6.75514030456543,
          distance: 504.1,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: -75.72223663330078,
          y: 6.75514030456543,
          distance: 508.7,
        },
      ],
    }),
    makeLine({
      title: '🇷🇺🇹🇷 Mt. Elbrus/Pontic Mountains',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 42.43996810913086,
          y: 43.34897232055664,
          distance: 483.1,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 42.43996810913086,
          y: 43.34897232055664,

          distance: 487.7,
        },
      ],
    }),
    makeLine({
      title: '🇮🇳 Patkai Hills/Gori Chen',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 97.25302124023438,
          y: 28.01169204711914,
          distance: 480.4,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 97.25302124023438,
          y: 28.01169204711914,

          distance: 480.3,
        },
      ],
    }),
    makeLine({
      title: '🇬🇹🇲🇽 Tajumulco/Nube Flane',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: -91.90559387207031,
          y: 15.041804313659668,
          distance: 477.9,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: -91.90559387207031,
          y: 15.041804313659668,

          distance: 484.7,
        },
      ],
    }),
    makeLine({
      title: '🇹🇯🇦🇫 Ravosang Pass/Ghowr',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 69.3552017211914,
          y: 39.26898956298828,
          distance: 475.3,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 69.3552017211914,
          y: 39.26898956298828,
          distance: 475.3,
        },
      ],
    }),
    makeLine({
      title: '🇵🇰🇺🇿 Noshaq/Aghbai Kalon',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 71.8376235961914,
          y: 36.432369232177734,
          distance: 470.1,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 71.8376235961914,
          y: 36.432369232177734,
          distance: 470.1,
        },
      ],
    }),
    makeLine({
      title: '🇦🇶 Antarctica',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 162.6785430908203,
          y: -78.0618667602539,
          distance: 463.8,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 162.6785430908203,
          y: -78.0618667602539,
          distance: 463.8,
        },
      ],
    }),
    makeLine({
      title: '🇷🇺🇮🇷 Balakuri/Sabalan',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 46.122562408447266,
          y: 42.21756362915039,
          distance: 462.9,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 46.122562408447266,
          y: 42.21756362915039,
          distance: 467.4,
        },
      ],
    }),
    makeLine({
      title: '🇵🇬 Kotewon/Suckling',
      numbers: [
        {
          runName: 'ryan-fullworld-raw',
          x: 146.864501953125,
          y: -6.220884323120117,
          distance: 451.6,
        },
        {
          runName: 'ryan-world2-run-refraction23',
          x: 146.864501953125,
          y: -6.220884323120117,
          distance: 451.8,
        },
      ],
    }),
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
