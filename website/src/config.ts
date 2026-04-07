import { LngLat } from 'maplibre-gl';
import { getPMTilesSource, getSubdomain } from './utils';

type Config = {
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
  };
};

export const worldConfig: Config = {
  map: {
    minZoom: 1.6,
    maxZoom: 16,
    startingZoom: 2.0,
    startingCentre: new LngLat(-5.0, 25.0),
  },
  heatmap: {
    averageVisibility: 700000.0,
  },
};

export const galianoConfig: Config = {
  map: {
    minZoom: 11,
    maxZoom: 16,
    startingZoom: 11.0,
    startingCentre: new LngLat(-123.445503, 48.934705),
  },
  heatmap: {
    averageVisibility: 900000.0,
  },
};

export function getViewViewRegion() {
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
  switch (getViewViewRegion()) {
    case 'galiano':
      return galianoConfig;
    default:
      return worldConfig;
  }
}
