import nprogress from 'accessible-nprogress';
import type { GeoTIFFImage } from 'geotiff';
import { fromUrl as geotiffFromURL } from 'geotiff';
import { LngLat } from 'maplibre-gl';
import proj4 from 'proj4';
import {
  aeqdProjectionString,
  CACHE_BUSTER,
  CDN_BUCKET,
  Log,
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

type Index = {
  centre: LngLat;
  width: number;
};

// Source of the longest lines COGs.
const LONGEST_LINES_COGS = getLongestLinesSource();
// Contents of index of Longest Lines COGs.
const cogsIndex: Map<string, Index> = new Map();
// Our local cache of COG files.
const cogs: Map<string, GeoTIFFImage> = new Map();

// Given a lon/lat coordinate, get the nearest actual COG point to it.
export async function getLongestLine(coordinate: LngLat) {
  nprogress.start();
  const candidates = await getLongestLines(coordinate);
  nprogress.done();

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
  const urls = await findNearestCOGURLs(coordinate);
  if (urls.length === 0) {
    console.warn(
      `${coordinate} is not within the radius of any Longest Line COGs`,
    );
    return;
  }

  const candidates: LongestLine[] = [];
  for (const url of urls) {
    const cog = await getCOG(url);
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
  const result = getPointCoordinate(cog, coordinate);
  if (result === undefined) {
    return;
  }
  const { x_point, y_point } = result;
  const flipper = cog.getWidth() - 1.0;
  const y_flipped = flipper - y_point;

  const { distance, angle } = await getPointFromRaster(cog, x_point, y_flipped);

  Log.debug('clicked at', coordinate);
  Log.debug('coordinate in raster', [x_point, y_flipped]);
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

  const raster = (await cog.readRasters({
    window: [x - around, y - around, x + around, y + around],
    width: around * 2 + 1,
    height: around * 2 + 1,
  })) as Float32Array[];

  let max = { distance: 0, angle: 0 };
  for (const row of raster) {
    for (const packed of row) {
      const packed_u32 = f32ToU32(packed);
      const distance = (packed_u32 >>> 10) & U22_MASK;
      const angle = packed_u32 & U10_MASK;
      if (distance > max.distance) {
        max = { distance, angle };
      }
    }
  }

  return max;
}

async function getCOG(url: string) {
  let cog = cogs.get(url);

  if (cog === undefined) {
    const tiff = await geotiffFromURL(url);
    const image = await tiff.getImage();
    cog = image;
    cogs.set(url, image);
  }

  return cog;
}

async function findNearestCOGURLs(coordinate: LngLat) {
  const indexURL = `${LONGEST_LINES_COGS}/index.txt${CACHE_BUSTER}`;
  if (cogsIndex.size === 0) {
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
  }

  Log.debug('Longest Lines index', cogsIndex);

  const urls = [];
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
      urls.push(`${LONGEST_LINES_COGS}/${filename}`);
      Log.debug(`✅ Longest Line COG found: ${urls}`);
    }
  }

  return urls;
}

// Convert lon/lat to COG-relative point.
function getPointCoordinate(image: GeoTIFFImage, coordinate: LngLat) {
  const geo = image.getGeoKeys();
  const projection = aeqdProjectionString(
    geo.ProjCenterLongGeoKey,
    geo.ProjCenterLatGeoKey,
  );
  const resolution = image.getResolution();

  const [x_metres, y_metres] = proj4(proj4.WGS84, projection, [
    coordinate.lng,
    coordinate.lat,
  ]);
  const offset = image.getWidth() / 2.0;
  const scale = Math.abs(resolution[0]);
  const x_point = Math.floor(x_metres / scale) + offset;
  const y_point = Math.floor(y_metres / scale) + offset;

  // TODO: update this to check outside the circle of the tile?
  if (
    x_point < 0 ||
    y_point < 0 ||
    x_point >= image.getWidth() ||
    y_point >= image.getHeight()
  ) {
    console.warn(
      `Clicked point ${x_point}/${y_point} is out of bounds ${image.getWidth()}x${image.getHeight()}`,
    );
    return;
  }

  return { x_point, y_point };
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
