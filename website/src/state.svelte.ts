import type { Map as MapLibre } from 'maplibre-gl';

let map: MapLibre | undefined;

export const state = $state({ map });
