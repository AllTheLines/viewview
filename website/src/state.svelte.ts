import type { Map as MapLibre } from 'maplibre-gl';
import { getConfig } from './config';
import type { LongestLine } from './lib/getLongestLine';
import type { Viewshed } from './lib/Viewshed';
import type { LongestLineH3 } from './lib/worldLines';

export type HeatmapConfig = {
  contrast: number;
  intensity: number;
};

export interface AppState {
  map: MapLibre | undefined;
  config: ReturnType<typeof getConfig>;
  worldLongestLines: LongestLineH3[] | undefined;
  longestLine: LongestLine | undefined;
  longestLineInViewport: LongestLineH3 | undefined;
  isFirstInteraction: boolean;
  bruteForceLoadingLine: boolean;
  heatmapConfig: HeatmapConfig;
  viewsheds: Viewshed[];
  isFlying: boolean;
  isSearchOpen: boolean;
  isInfoOpen: boolean;
}

export const state = $state<AppState>({
  map: undefined,
  config: getConfig(),
  worldLongestLines: undefined,
  longestLine: undefined,
  longestLineInViewport: undefined,
  isFirstInteraction: false,
  bruteForceLoadingLine: false,
  heatmapConfig: {
    contrast: 1 - 0.45,
    intensity: 1 - 0.5,
  },
  viewsheds: [],
  isFlying: false,
  isSearchOpen: false,
  isInfoOpen: true,
});
