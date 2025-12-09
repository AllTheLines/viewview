import type { Map as MapLibre } from 'maplibre-gl';
import type { LongestLine } from './getLongestLine';

let map: MapLibre | undefined;
let longestLine: LongestLine | undefined;
const isFirstInteraction = false;

export const state = $state({ map, longestLine, isFirstInteraction });
