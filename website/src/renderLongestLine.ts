import {
  type GeoJSONFeature,
  type GeoJSONSource,
  LngLat,
  type MapMouseEvent,
} from 'maplibre-gl';
import proj4 from 'proj4';
import { getLongestLine } from './getLongestLine.ts';
import { state } from './state.svelte.ts';
import { aeqdProjectionString, toRadians } from './utils.ts';

// Inherited from the TVS algorithm. It's to counter unfavourable floating point rounding.
const ANGLE_SHIFT = 0.0001;

export function setup() {
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
      'fill-color': '#657DA4',
      'fill-outline-color': '#253B61',
      'fill-opacity': 0.7,
    },
  });

  state.map?.on('click', async (event) => {
    if (!state.map) {
      return;
    }

    render(event);
  });
}

async function render(event: MapMouseEvent) {
  const longest_line = await getLongestLine(event.lngLat);
  if (longest_line === undefined) {
    return;
  }

  state.longest_line = longest_line;

  if (import.meta.env.DEV) {
    console.log(longest_line);
  }

  // TODO: Why do we have to take away from 360? I suspect it's because forward and backward
  // lines are the wrong way round?
  longest_line.angle = 360 - longest_line.angle + ANGLE_SHIFT;

  const θ = toRadians(longest_line.angle);
  const dx = longest_line.distance * Math.cos(θ);
  const dy = longest_line.distance * Math.sin(θ);
  const rotatedClockwiseAEQD = rotate(dx, dy, -0.5);
  const rotatedAntiAEQD = rotate(dx, dy, +0.5);

  const aeqd = aeqdProjectionString(event.lngLat.lng, event.lngLat.lat);
  const unrotated = proj4(aeqd, proj4.WGS84, [dx, dy]);
  longest_line.from = event.lngLat;
  longest_line.to = new LngLat(unrotated[0], unrotated[1]);
  state.longest_line = longest_line;

  const rotatedClockwiseLonLat = proj4(aeqd, proj4.WGS84, rotatedClockwiseAEQD);
  const rotatedAntiLonLat = proj4(
    aeqd,
    '+proj=longlat +datum=WGS84 +no_defs',
    rotatedAntiAEQD,
  );
  const viewCoordinates = [
    event.lngLat.toArray(),
    rotatedClockwiseLonLat,
    rotatedAntiLonLat,
    event.lngLat.toArray(),
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
}

// Rotate a coordinate around the origin.
function rotate(x: number, y: number, degrees: number) {
  const θ = degrees * (Math.PI / 180);
  const cos = Math.cos(θ);
  const sin = Math.sin(θ);
  return [x * cos - y * sin, x * sin + y * cos];
}
