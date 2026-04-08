import { LngLat } from 'maplibre-gl';
import type { AppState } from '../state.svelte';

export function transformConstrain(state: AppState) {
  return (lngLat: LngLat, zoom: number) => {
    return transformConstrainInner(lngLat, zoom, state);
  };
}

// Custom map bounds that allows hiding most of Antartica, whilst still allowing infinite horizontal
// scroll.
//
// For an official fix, follow: https://github.com/maplibre/maplibre-gl-js/issues/6148
function transformConstrainInner(
  lngLat: LngLat,
  zoom: number,
  state: AppState,
) {
  const latitudeToMercatorY = (latitude: number) => {
    return (
      0.5 -
      (0.25 *
        Math.log(
          (1 + Math.sin((latitude * Math.PI) / 180)) /
            (1 - Math.sin((latitude * Math.PI) / 180)),
        )) /
        Math.PI
    );
  };

  const mercatorYToLatitude = (mercatorY: number) => {
    return (
      (360 / Math.PI) * Math.atan(Math.exp((0.5 - mercatorY) * 2 * Math.PI)) -
      90
    );
  };

  const viewportHeight = state.map?.getContainer().clientHeight;
  if (viewportHeight === undefined) {
    return {
      center: state.config.map.startingCentre,
      zoom: state.config.map.startingZoom,
    };
  }

  const upperLatitudeBound = 85;
  const lowerLatitudeBound = -80;

  const worldSize = 512 * 2 ** zoom;
  const mercatorYOffset = viewportHeight / 2 / worldSize;

  const maxMercatorY = latitudeToMercatorY(upperLatitudeBound);
  const maxLatitude = mercatorYToLatitude(maxMercatorY + mercatorYOffset);
  const minMercatorY = latitudeToMercatorY(lowerLatitudeBound);
  const minLatitude = mercatorYToLatitude(minMercatorY - mercatorYOffset);

  const latitude = Math.max(minLatitude, Math.min(maxLatitude, lngLat.lat));

  return {
    center: new LngLat(lngLat.lng, latitude),
    zoom: Math.max(
      state.config.map.minZoom,
      Math.min(state.config.map.maxZoom, zoom),
    ),
  };
}
