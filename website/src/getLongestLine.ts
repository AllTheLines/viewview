import nprogress from 'accessible-nprogress';
import type { GeoTIFFImage } from 'geotiff';
import { fromUrl as geotiffFromURL } from 'geotiff';
import { LngLat } from 'maplibre-gl';
import proj4 from 'proj4';
import { aeqdProjectionString, BUCKET, Log } from './utils';

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
  const cog = await getCOG(coordinate);
  if (cog === undefined) {
    return;
  }

  const result = getPointCoordinate(cog, coordinate);
  if (result === undefined) {
    return;
  }
  const { x_point, y_point } = result;

  nprogress.start();
  const raster = (await cog.readRasters({
    window: [x_point, y_point, x_point + 1, y_point + 1],
    width: 1,
    height: 1,
  })) as Float32Array[];
  nprogress.done();

  const packed = raster[0][0];
  const packed_u32 = f32ToU32(packed);
  const distance = (packed_u32 >>> 10) & U22_MASK;
  const angle = packed_u32 & U10_MASK;

  if (import.meta.env.DEV) {
    console.log('clicked at', coordinate);
    console.log('coordinate in raster', [x_point, y_point]);
    console.log('packed:', packed_u32, u32BitsToString(packed_u32));
    console.log('distance:', distance, u32BitsToString(distance));
    console.log('angle:', angle, u32BitsToString(angle));
  }

  if (distance === 0 && angle === 0) {
    if (import.meta.env.DEV) {
      console.log('distance and angle are both 0');
    }
    return;
  }

  return { distance, angle } as LongestLine;
}

async function getCOG(coordinate: LngLat) {
  const url = await findNearestCOGURL(coordinate);
  if (url === undefined) {
    console.warn(
      `${coordinate} is not within the radius of any Longest Line COGs`,
    );
    return;
  }

  let cog = cogs.get(url);

  if (cog === undefined) {
    nprogress.start();
    const tiff = await geotiffFromURL(url);
    const image = await tiff.getImage();
    cog = image;
    cogs.set(url, image);
    nprogress.done();
  }

  return cog;
}

async function findNearestCOGURL(coordinate: LngLat) {
  const indexURL = `${LONGEST_LINES_COGS}/index.txt`;
  if (cogsIndex.size === 0) {
    Log.debug(`Fetching Longest Line COGs file: ${indexURL}`);

    nprogress.start();
    const result = await fetch(indexURL);
    if (!result.ok) throw new Error(result.status.toString());
    const contents = await result.text();
    nprogress.done();

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

  for (const [filename, cog] of cogsIndex) {
    const distance = coordinate.distanceTo(cog.centre);
    const radius = cog.width / 2;
    if (distance < radius) {
      const url = `${LONGEST_LINES_COGS}/${filename}`;
      Log.debug(`Longest Line COG found: ${url}`);
      return url;
    }
  }
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
  const offset = image.getWidth() / 2;
  const x_point = Math.floor(x_metres / resolution[0]) + offset;
  const y_point = Math.floor(y_metres / resolution[1]) + offset;

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
  if (!import.meta.env.DEV) {
    return `${BUCKET}/longest_lines`;
  } else {
    return `/longest_lines`;
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
