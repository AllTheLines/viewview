import { LngLat } from 'maplibre-gl';
import { getPMTilesSource, getSubdomain } from './lib/utils';

export type ViewViewProject = 'world' | 'galiano';

type Config = {
  project: ViewViewProject;
  map: {
    minZoom: number;
    maxZoom: number;
    startingZoom: number;
    startingCentre: LngLat;
  };
  heatmap: {
    // The average surface area visibile from a point far out at sea, where it can only see sea.
    // This is used to fill regions for which there is no elevation data.
    averageVisibility: number;
    defaultContrast: number;
    defaultIntensity: number;
  };
};

export const worldConfig: Config = {
  project: 'world',
  map: {
    minZoom: 1.6,
    maxZoom: 16,
    startingZoom: 2.0,
    startingCentre: new LngLat(-5.0, 25.0),
  },
  heatmap: {
    averageVisibility: 700000.0,
    defaultContrast: 1 - 0.45,
    defaultIntensity: 1 - 0.5,
  },
};

export const galianoConfig: Config = {
  project: 'galiano',
  map: {
    minZoom: 11,
    maxZoom: 20,
    startingZoom: 11.5,
    startingCentre: new LngLat(-123.445503, 48.934705),
  },
  heatmap: {
    averageVisibility: 39000000.0,
    defaultContrast: 1 - 0.09,
    defaultIntensity: 1 - 0.6,
  },
};

export function getViewViewProject() {
  const galianoSignature = 'galiano';
  if (
    getSubdomain()?.includes(galianoSignature) ||
    getPMTilesSource()?.includes(galianoSignature)
  ) {
    console.log('Using Galiano config');
    return 'galiano';
  }

  return 'world';
}

export function getConfig() {
  switch (getViewViewProject()) {
    case 'galiano':
      return galianoConfig;
    default:
      return worldConfig;
  }
}
