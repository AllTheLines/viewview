import type { Map as MapLibre } from 'maplibre-gl';
import { getConfig } from './config';
import type { IndexedTile, LongestLine } from './lib/getLongestLine';
import type { Viewshed } from './lib/Viewshed';
import type { LongestLineH3 } from './lib/worldLines';

const DEV_KEY = 'dev-state:app';

const config = getConfig();

export type RunName =
  | 'ryan-fullworld-raw'
  | 'ryan-world2-run-refraction23'
  | 'galiano';

export type HeatmapConfig = {
  contrast: number;
  intensity: number;
  longestLinesRun: RunName;
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
  cogsIndex: Map<string, IndexedTile>;
  viewsheds: Viewshed[];
  isFlying: boolean;
  isSearchOpen: boolean;
  isInfoOpen: boolean;
  isSettingsOpen: boolean;
  isTopLinesOpen: boolean;
}

export const state = $state<AppState>({
  map: undefined,
  config,
  worldLongestLines: undefined,
  longestLine: undefined,
  longestLineInViewport: undefined,
  isFirstInteraction: false,
  bruteForceLoadingLine: false,
  heatmapConfig: {
    contrast: config.heatmap.defaultContrast,
    intensity: config.heatmap.defaultIntensity,
    longestLinesRun: config.heatmap.defaultLongestLinesRun,
  },
  cogsIndex: new Map(),
  viewsheds: [],
  isFlying: false,
  isSearchOpen: false,
  isInfoOpen: true,
  isSettingsOpen: false,
  isTopLinesOpen: false,
});

// This is just for development. So you don't have to keep re-opening models etc
const persistedKeys = [
  'isFirstInteraction',
  'heatmapConfig',
  'isSearchOpen',
  'isInfoOpen',
  'isSettingsOpen',
  'isTopLinesOpen',
] as const satisfies readonly (keyof AppState)[];

if (import.meta.env.DEV) {
  // Restore before anything reads state
  try {
    // Session storage is per-tab. So just close the tab and reopen in another tab to reset the state.
    const raw = sessionStorage.getItem(DEV_KEY);
    if (raw) {
      const saved = JSON.parse(raw);
      for (const key of persistedKeys) {
        if (key in saved) {
          // biome-ignore lint/suspicious/noExplicitAny: key-by-key restore
          (state as any)[key] = saved[key];
        }
      }
    }
  } catch {}

  // Save on change
  $effect.root(() => {
    $effect(() => {
      try {
        const subset = Object.fromEntries(
          persistedKeys.map((key) => [key, state[key]]),
        );
        sessionStorage.setItem(
          DEV_KEY,
          JSON.stringify($state.snapshot(subset)),
        );
      } catch {}
    });
  });
}
