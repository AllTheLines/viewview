import { LngLat, LngLatBounds } from 'maplibre-gl';

export const CDN_BUCKET = 'https://cdn.alltheviews.world';
export const MAP_SERVER = 'https://map.alltheviews.world';
export const WORLD_PMTILES = 'world';
export const PMTILES_SERVER = `${MAP_SERVER}/runs/0.1/pmtiles/${WORLD_PMTILES}`;

export const VERSION = '0.1';
export const EARTH_RADIUS = 6371_000.0;

export const Log = {
  // biome-ignore lint/suspicious/noExplicitAny: needed for debugging.
  debug: (...data: any[]) => {
    if (import.meta.env.DEV) {
      console.debug(...data);
    }
  },

  // biome-ignore lint/suspicious/noExplicitAny: needed for debugging.
  trace: (...data: any[]) => {
    if (import.meta.env.DEV) {
      console.trace(...data);
    }
  },
};

export function tileKey(z: number, x: number, y: number) {
  return `${z}/${x}/${y}`;
}

export function getParentTile(z: number, x: number, y: number) {
  if (z === 0) {
    return null;
  }

  const parentZ = z - 1;
  const parentX = Math.floor(x / 2);
  const parentY = Math.floor(y / 2);
  return { z: parentZ, x: parentX, y: parentY };
}

export function tileToLatLonBounds(z: number, x: number, y: number) {
  const n = 2 ** z;

  const west = (x / n) * 360 - 180;
  const east = ((x + 1) / n) * 360 - 180;
  const north =
    (Math.atan(Math.sinh(Math.PI * (1 - (2 * y) / n))) * 180) / Math.PI;
  const south =
    (Math.atan(Math.sinh(Math.PI * (1 - (2 * (y + 1)) / n))) * 180) / Math.PI;

  const sw = new LngLat(west, south);
  const ne = new LngLat(east, north);
  return new LngLatBounds(sw, ne);
}

export function isTileIntersectingBounds(
  tile: LngLatBounds,
  bounds: LngLatBounds,
) {
  if (tile._ne.lng < bounds._sw.lng) return false;
  if (tile._sw.lng > bounds._ne.lng) return false;
  if (tile._ne.lat < bounds._sw.lat) return false;
  if (tile._sw.lat > bounds._ne.lat) return false;
  return true;
}

export function aeqdProjectionString(longitude: number, latitude: number) {
  return (
    `+proj=aeqd ` +
    `+lon_0=${longitude} ` +
    `+lat_0=${latitude} ` +
    `+x_0=0 +y_0=0 +datum=WGS84 +units=m +no_defs`
  );
}

export function toRadians(degrees: number) {
  return degrees * (Math.PI / 180);
}

export function toDegrees(radians: number) {
  return radians * (180 / Math.PI);
}

export function lonLatRound(lonlat: LngLat) {
  const precision = 6;
  return [lonlat.lng.toPrecision(precision), lonlat.lat.toPrecision(precision)];
}

export function packFloatToU8s(float: number) {
  const buffer = new ArrayBuffer(4);
  new Float32Array(buffer)[0] = float;
  const u8s = new Uint8Array(buffer);
  return u8s;
}

export function computeBBox(coordinates: number[][]) {
  let minLng = Infinity,
    minLat = Infinity;
  let maxLng = -Infinity,
    maxLat = -Infinity;

  for (const coordinate of coordinates) {
    const lngLat = new LngLat(coordinate[0], coordinate[1]);
    minLng = Math.min(minLng, lngLat.lng);
    minLat = Math.min(minLat, lngLat.lat);
    maxLng = Math.max(maxLng, lngLat.lng);
    maxLat = Math.max(maxLat, lngLat.lat);
  }

  return new LngLatBounds([minLng, minLat, maxLng, maxLat]);
}
