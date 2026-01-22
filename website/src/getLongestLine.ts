import type { GeoTIFFImage } from 'geotiff';
import { fromUrl as geotiffFromURL } from 'geotiff';
import { LngLat, type LngLatBounds } from 'maplibre-gl';
import proj4 from 'proj4';
import {
  aeqdProjectionString,
  CACHE_BUSTER,
  CDN_BUCKET,
  clamp,
  endLoadingSpinner,
  Log,
  startLoadingSpinner,
  VERSION,
} from './utils';

// Masks for unpacking bit-packed line of sight data.
const U22_MASK = (1 << 22) - 1;
const U10_MASK = (1 << 10) - 1;

export type LongestLine = {
  distance: number;
  angle: number;
  from: LngLat;
  to: LngLat;
};

type IndexedTile = {
  centre: LngLat;
  width: number;
};

// Source of the longest lines COGs.
const LONGEST_LINES_COGS = getLongestLinesSource();
// Contents of index of Longest Lines COGs.
const cogsIndex: Map<string, IndexedTile> = new Map();
// Our local cache of COG files.
const cogs: Map<string, GeoTIFFImage> = new Map();

// Given a lon/lat coordinate, get the nearest actual COG point to it.
export async function getLongestLine(coordinate: LngLat) {
  startLoadingSpinner();
  const candidates = await getLongestLines(coordinate);
  endLoadingSpinner();

  if (candidates === undefined || candidates.length === 0) {
    return;
  }

  let longest: LongestLine | undefined;
  for (const candidate of candidates) {
    if (longest === undefined) {
      longest = candidate;
      continue;
    }
    if (candidate.distance > longest?.distance) {
      longest = candidate;
    }
  }

  Log.debug(`Using longest line: `, longest);

  return longest;
}

async function getLongestLines(coordinate: LngLat) {
  const cogFilenames = await findNearestCOGURLs(coordinate);
  if (cogFilenames.length === 0) {
    console.warn(
      `${coordinate} is not within the radius of any Longest Line COGs`,
    );
    return;
  }

  const candidates: LongestLine[] = [];
  for (const filename of cogFilenames) {
    const cog = await getCOG(filename);
    if (cog === undefined) {
      continue;
    }
    const candidate = await getLongestLineCandidate(cog, coordinate);
    if (candidate) {
      candidates.push(candidate);
    }
  }

  return candidates;
}

async function getLongestLineCandidate(cog: GeoTIFFImage, coordinate: LngLat) {
  const { x_point, y_point } = convertLngLatToRasterXY(cog, coordinate);
  // TODO: update this to check outside the _circle_ of the tile?
  if (
    x_point < 0 ||
    y_point < 0 ||
    x_point >= cog.getWidth() ||
    y_point >= cog.getHeight()
  ) {
    console.warn(
      `Clicked point ${x_point}/${y_point} is out of bounds ${cog.getWidth()}x${cog.getHeight()}`,
    );
    return;
  }

  Log.debug('clicked at', coordinate);
  Log.debug('coordinate in raster', [x_point, y_point]);

  const max = await getPointFromRaster(cog, x_point, y_point);
  if (max === undefined) {
    Log.debug("Couldn't find longest line at clicked point");
    return;
  }

  const { distance, angle } = max;

  Log.debug('distance:', distance, u32BitsToString(distance));
  Log.debug('angle:', angle, u32BitsToString(angle));

  if (distance === 0 && angle === 0) {
    Log.debug('distance and angle are both 0');
    return;
  }

  return { distance, angle } as LongestLine;
}

async function getPointFromRaster(cog: GeoTIFFImage, x: number, y: number) {
  // We need to find the longest line in a 5x5 grid to get around precision loss.
  const around = 2;

  const extent = [x - around, y - around, x + around, y + around];

  const max = findLongestInWindow(cog, extent);
  return max;
}

export async function findLongestInWindow(cog: GeoTIFFImage, window: number[]) {
  Log.debug(
    `Raster size: ${cog.getWidth()}x${cog.getHeight()}.`,
    'Fetching data from raster using window:',
    window,
  );
  const raster = (await cog.readRasters({
    window,
  })) as Float32Array[];
  const packedLines = raster[0];

  const windowOriginX = window[0];
  const windowOriginY = window[1];
  const width = window[2] - window[0];

  let max:
    | { distance: number; angle: number; x: number; y: number }
    | undefined;
  let index = 0;
  for (const packedLine of packedLines) {
    const packed_u32 = f32ToU32(packedLine);
    const distance = (packed_u32 >>> 10) & U22_MASK;
    const angle = packed_u32 & U10_MASK;
    if (max === undefined || distance > max.distance) {
      const x = windowOriginX + Math.floor(index / width);
      const y = windowOriginY + (index % width);
      max = { distance, angle, x, y };
    }
    index += 1;
  }

  return max;
}

async function getCOG(filename: string) {
  const url = `${LONGEST_LINES_COGS}/${filename}`;
  let cog = cogs.get(url);

  if (cog === undefined) {
    const tiff = await geotiffFromURL(url);
    const image = await tiff.getImage();
    cog = image;
    cogs.set(url, image);
  }

  return cog;
}

async function ensureLongestLinesIndexLoaded() {
  if (cogsIndex.size > 0) {
    return;
  }
  const indexURL = `${LONGEST_LINES_COGS}/index.txt${CACHE_BUSTER}`;
  Log.debug(`Fetching Longest Line COGs file: ${indexURL}`);

  const result = await fetch(indexURL);
  if (!result.ok) throw new Error(result.status.toString());
  const contents = await result.text();

  for (const line of contents.split('\n')) {
    const lineParts = line.split(' ');
    if (lineParts.length !== 2) {
      continue;
    }
    const filename = lineParts[0];
    const lonLatParts = filename.replace('.tiff', '').split('_');
    const centre = new LngLat(
      parseFloat(lonLatParts[0]),
      parseFloat(lonLatParts[1]),
    );
    const width = parseInt(lineParts[1], 10);
    cogsIndex.set(filename, { centre, width });
  }

  Log.debug('Longest Lines index', cogsIndex);
}

async function findNearestCOGURLs(coordinate: LngLat) {
  await ensureLongestLinesIndexLoaded();

  const cogFilenames = [];
  for (const [filename, cog] of cogsIndex) {
    const distance = coordinate.distanceTo(cog.centre);
    const scale = 100; // TODO: I thought we decided to set this in the indexer?
    const radius = cog.width / 2 / scale;
    Log.debug(
      `👀 Checking Longest Line COG: ${filename}`,
      `with centre: ${cog.centre} and radius ${radius}`,
      `Distance from click: ${distance}`,
    );
    if (distance < radius) {
      cogFilenames.push(filename);
      Log.debug(`✅ Longest Line COG found: ${cogFilenames}`);
    }
  }

  return cogFilenames;
}

export async function findTilesIntersectingViewport(
  viewportBounds: LngLatBounds,
) {
  await ensureLongestLinesIndexLoaded();
  const cogsNearby = [];
  for (const [filename, tile] of cogsIndex) {
    if (isTileIntersectingViewport(tile, viewportBounds)) {
      Log.debug('Viewport-intersecting tile found:', tile);
      const cog = await getCOG(filename);
      cogsNearby.push(cog);
    }
  }
  return cogsNearby;
}

function isTileIntersectingViewport(
  tile: IndexedTile,
  viewportBounds: LngLatBounds,
) {
  // TODO: why divide by 100?? The width is already in metres right?? And the distance calculation
  // is in meters, so..?
  const radius = tile.width / 2.0 / 100.0;

  // Closest point on square to circle center
  const closestX = clamp(
    tile.centre.lng,
    viewportBounds.getSouthWest().lng,
    viewportBounds.getSouthEast().lng,
  );
  const closestY = clamp(
    tile.centre.lat,
    viewportBounds.getSouthWest().lat,
    viewportBounds.getNorthWest().lat,
  );

  // Squared distance from circle center to that point
  const distance = tile.centre.distanceTo(new LngLat(closestX, closestY));
  const distanceSquared = distance * distance;

  return distanceSquared <= radius * radius;
}

// Convert lon/lat to COG-relative point.
export function convertLngLatToRasterXY(cog: GeoTIFFImage, coordinate: LngLat) {
  const geo = cog.getGeoKeys();
  const projection = aeqdProjectionString(
    geo.ProjCenterLongGeoKey,
    geo.ProjCenterLatGeoKey,
  );
  const resolution = cog.getResolution();
  const maxIndex = cog.getWidth() - 1.0;

  const [x_metres, y_metres] = proj4(proj4.WGS84, projection, [
    coordinate.lng,
    coordinate.lat,
  ]);
  const offset = maxIndex / 2.0;
  const scale = Math.abs(resolution[0]);
  const x_unclamped = Math.floor(x_metres / scale + offset);
  const y_unclamped = Math.floor(y_metres / scale + offset);
  const x_point = clamp(x_unclamped, 0, maxIndex);
  const y_point = maxIndex - clamp(y_unclamped, 0, maxIndex);

  return { x_point, y_point };
}

// Convert a raster's x,y pixel coordinate to long/lat.
export function convertRasterXYToLngLat(
  cog: GeoTIFFImage,
  x: number,
  y: number,
) {
  const geo = cog.getGeoKeys();
  const projection = aeqdProjectionString(
    geo.ProjCenterLongGeoKey,
    geo.ProjCenterLatGeoKey,
  );
  const resolution = cog.getResolution();
  const scale = Math.abs(resolution[0]);
  const maxIndex = cog.getWidth() - 1.0;
  const offset = maxIndex / 2.0;
  const y_flipped = maxIndex - y;

  const x_metric = (x - offset) * scale;
  const y_metric = (y_flipped - offset) * scale;
  const [lng, lat] = proj4(projection, proj4.WGS84, [x_metric, y_metric]);
  console.log(x, y, x_metric, y_metric, lng, lat);

  return new LngLat(lng, lat);
}

function getLongestLinesSource() {
  const params = new URLSearchParams(self.location.search);
  const source = params.get('longest_lines');
  if (!source) {
    if (!import.meta.env.DEV) {
      return `${CDN_BUCKET}/runs/${VERSION}/longest_lines_cogs`;
    } else {
      return `/longest_lines`;
    }
  } else {
    return source;
  }
}

// Transmute `f32` to `u32`.
function f32ToU32(float: number) {
  const buffer = new ArrayBuffer(4);
  new Float32Array(buffer)[0] = float;
  const unsigned = new Uint32Array(buffer)[0] >>> 0;
  return unsigned;
}

// For debugging.
function u32BitsToString(u32: number) {
  return (u32 >>> 0).toString(2).padStart(32, '0');
}
