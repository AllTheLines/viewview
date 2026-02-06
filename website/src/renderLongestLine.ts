import { type GeoJSONFeature, type GeoJSONSource, LngLat } from 'maplibre-gl';
import proj4 from 'proj4';
import { navigate } from 'svelte5-router';
import { getLongestLine } from './getLongestLine.ts';
import { state } from './state.svelte.ts';
import {
  aeqdProjectionString,
  computeBBox,
  disablePointer,
  toRadians,
} from './utils.ts';

// Inherited from the TVS algorithm. It's to counter unfavourable floating point rounding.
const ANGLE_SHIFT = 0.0001;

export function setupLongestLines(coordFromURL: string | undefined) {
  state.map?.addSource('longest-line', {
    type: 'geojson',
    data: {
      type: 'FeatureCollection',
      features: [],
    },
  });

  state.map?.addLayer({
    id: 'longest-line-fill',
    type: 'fill',
    source: 'longest-line',
    paint: {
      'fill-color': '#5BC0EB',
      'fill-outline-color': '#ffffff',
      'fill-opacity': 0.8,
    },
  });

  state.map?.on('click', async (event) => {
    if (!state.map) {
      return;
    }

    render(event.lngLat);
    const coord = event.lngLat;

    navigate(longestLineURL(coord.lng, coord.lat));
  });

  if (coordFromURL?.startsWith('longest/')) {
    const coord = extractCoordFromURL(coordFromURL);
    render(coord);
  }
}

function extractCoordFromURL(coordFromURL: string) {
  const parts = coordFromURL.replace('longest/', '').split('_');
  const lng = parseFloat(parts[0]);
  const lat = parseFloat(parts[1]);
  const coord = new LngLat(lng, lat);
  return coord;
}

export async function render(lngLat: LngLat) {
  const longest_line = await getLongestLine(lngLat);
  if (longest_line === undefined) {
    return;
  }

  state.longestLine = longest_line;

  if (import.meta.env.DEV) {
    console.log(longest_line);
  }

  longest_line.angle = longest_line.angle + ANGLE_SHIFT;

  const θ = toRadians(longest_line.angle);
  const dx = longest_line.distance * Math.cos(θ);
  const dy = longest_line.distance * Math.sin(θ);
  const rotatedClockwiseAEQD = rotate(dx, dy, -0.5);
  const rotatedAntiAEQD = rotate(dx, dy, +0.5);

  const aeqd = aeqdProjectionString(lngLat.lng, lngLat.lat);
  const unrotated = proj4(aeqd, proj4.WGS84, [dx, dy]);
  longest_line.from = lngLat;
  longest_line.to = new LngLat(unrotated[0], unrotated[1]);
  state.longestLine = longest_line;

  const rotatedClockwiseLonLat = proj4(aeqd, proj4.WGS84, rotatedClockwiseAEQD);
  const rotatedAntiLonLat = proj4(
    aeqd,
    '+proj=longlat +datum=WGS84 +no_defs',
    rotatedAntiAEQD,
  );
  const viewCoordinates = [
    lngLat.toArray(),
    rotatedClockwiseLonLat,
    rotatedAntiLonLat,
    lngLat.toArray(),
  ];

  const longestLineGeoJSON = {
    type: 'Feature',
    geometry: {
      type: 'Polygon',
      coordinates: [viewCoordinates],
    },
    properties: {},
  } as GeoJSONFeature;

  const source = state.map?.getSource('longest-line') as GeoJSONSource;

  source?.setData(longestLineGeoJSON);

  state.map?.fitBounds(computeBBox(viewCoordinates), {
    padding: 100,
    duration: 1000,
  });
  state.isFlying = true;
  disablePointer();
}

// Rotate a coordinate around the origin.
function rotate(x: number, y: number, degrees: number) {
  const θ = degrees * (Math.PI / 180);
  const cos = Math.cos(θ);
  const sin = Math.sin(θ);
  return [x * cos - y * sin, x * sin + y * cos];
}

export function longestLineURL(lon: number, lat: number) {
  return `/longest/${lon}_${lat}${window.location.search}`;
}
