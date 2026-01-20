import type { Map as MapLibre } from 'maplibre-gl';
import type { LongestLine } from './getLongestLine';
import type { LongestLineH3 } from './worldLines';

export type HeatmapConfig = {
  contrast: number;
  intensity: number;
};

let map: MapLibre | undefined;
let worldLongestLines: LongestLineH3[] | undefined;
let longestLine: LongestLine | undefined;
let longestLineInViewport: LongestLineH3 | undefined;
const isFirstInteraction = false;
const bruteForceLoadingLine = false;
const heatmapConfig: HeatmapConfig = {
  contrast: 1 - 0.45,
  intensity: 1 - 0.5,
};

export const state = $state({
  map,
  worldLongestLines,
  longestLine,
  longestLineInViewport,
  isFirstInteraction,
  bruteForceLoadingLine,
  heatmapConfig,
});
