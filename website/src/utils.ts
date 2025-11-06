import { LngLat, LngLatBounds } from 'maplibre-gl';

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
