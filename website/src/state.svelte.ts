import type { Map as MapLibre } from 'maplibre-gl';
import type { LongestLine } from './getLongestLine';

let map: MapLibre | undefined;
let longest_line: LongestLine | undefined;

export const state = $state({ map, longest_line });
