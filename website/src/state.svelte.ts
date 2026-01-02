import type { Map as MapLibre } from 'maplibre-gl';
import type { LongestLine } from './getLongestLine';
import type { LongestLineH3 } from './worldLines';

let map: MapLibre | undefined;
let worldLongestLines: LongestLineH3[] | undefined;
let longestLine: LongestLine | undefined;
let longestLineInViewport: LongestLineH3 | undefined;
const isFirstInteraction = false;

export const state = $state({
  map,
  worldLongestLines,
  longestLine,
  longestLineInViewport,
  isFirstInteraction,
});
