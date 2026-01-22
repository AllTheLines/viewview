import nprogress from 'accessible-nprogress';
import type { GeoTIFFImage } from 'geotiff';
import { LngLat, type LngLatBounds } from 'maplibre-gl';
import {
  convertLngLatToRasterXY,
  convertRasterXYToLngLat,
  findLongestInWindow,
  findTilesIntersectingViewport,
} from './getLongestLine';
import { longestLineURL } from './renderLongestLine';
import { state } from './state.svelte';
import { CACHE_BUSTER, CDN_BUCKET, Log, VERSION } from './utils';

/// The filename for the longest lines grid.
const LONGEST_LINES_GRIDED_FILENAME = 'longest_lines_grided.bin';

export class LongestLineH3 {
  coordinate: LngLat;
  distance: number;

  constructor(coordinate: LngLat, distance: number) {
    this.coordinate = coordinate;
    this.distance = distance;
  }

  toURL() {
    return longestLineURL(this.coordinate.lng, this.coordinate.lat);
  }

  toDistance() {
    const distance = this.distance / 1000;
    return `${distance}km`;
  }
}

export async function loadH3Lines() {
  let indexURL = `${CDN_BUCKET}/runs/${VERSION}/${LONGEST_LINES_GRIDED_FILENAME}${CACHE_BUSTER}`;
  const params = new URLSearchParams(self.location.search);
  const source = params.get('grid');
  if (source) {
    indexURL = source;
  }
  Log.debug(`Fetching Longest Lines H3 file: ${indexURL}`);
  const result = await fetch(indexURL);

  if (!result.ok) throw new Error(result.status.toString());
  const binary = await result.arrayBuffer();
  const dataView = new DataView(binary);

  const itemSize = 12; // 2*f32 (8 bytes) + u32 (4 bytes)
  const longestLine: LongestLineH3[] = [];
  for (
    let offset = 0;
    offset + itemSize <= dataView.byteLength;
    offset += itemSize
  ) {
    const x = dataView.getFloat32(offset + 0, true); // little-endian
    const y = dataView.getFloat32(offset + 4, true);
    const distance = dataView.getUint32(offset + 8, true);
    longestLine.push(new LongestLineH3(new LngLat(x, y), distance));
  }

  longestLine.sort((left, right) => right.distance - left.distance);

  state.worldLongestLines = longestLine;
}

export async function findLongestLineInBoundsFromGrid(bounds: LngLatBounds) {
  if (state.worldLongestLines === undefined) {
    await loadH3Lines();
  }

  if (state.worldLongestLines === undefined) {
    return;
  }

  let max: LongestLineH3 | undefined;

  for (const line of state.worldLongestLines) {
    if (bounds.contains(line.coordinate)) {
      if (max === undefined || line.distance > max.distance) {
        max = line;
      }
    }
  }

  if (max !== undefined) {
    Log.debug('Found longest line in viewport via grid:', max?.distance);
  } else {
    Log.debug("Couldn't find viewport longest line in grid");
  }

  return max;
}

export async function findLongestLineInBoundsBruteForce(bounds: LngLatBounds) {
  let max: LongestLineH3 | undefined;

  nprogress.start();

  const cogs = await findTilesIntersectingViewport(bounds);

  for (const cog of cogs) {
    const extent = convertViewportBoundsToCOGWindow(bounds, cog);
    const longest = await findLongestInWindow(cog, extent);
    if (longest === undefined) {
      Log.debug(`Couldn't find longest line in cog: ${cog}`);
      continue;
    }
    const coordinate = convertRasterXYToLngLat(cog, longest.x, longest.y);
    if (max === undefined || longest.distance > max.distance) {
      max = new LongestLineH3(coordinate, longest.distance);
    }
  }

  if (max !== undefined) {
    Log.debug('Found longest line in viewport via brute force:', max.distance);
  } else {
    Log.debug(
      `Couldn't find longest line in viewport ${bounds} via brute force`,
    );
  }

  nprogress.done();
  return max;
}

function convertViewportBoundsToCOGWindow(
  bounds: LngLatBounds,
  cog: GeoTIFFImage,
) {
  const northWest = convertLngLatToRasterXY(cog, bounds.getNorthWest());
  const southEast = convertLngLatToRasterXY(cog, bounds.getSouthEast());

  return [
    northWest.x_point,
    northWest.y_point,
    southEast.x_point,
    southEast.y_point,
  ];
}
