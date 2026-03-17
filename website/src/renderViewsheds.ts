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

export function setupViewsheds() {
  state.map?.addSource('viewshed', {
    type: 'geojson',
    data: {
      type: 'FeatureCollection',
      features: [],
    },
  });

  state.map?.addLayer({
    id: 'viewshed-fill',
    type: 'fill',
    source: 'viewshed',
    paint: {
      'fill-color': '#ff0000',
      'fill-outline-color': '#ffffff',
      'fill-opacity': 0.8,
    },
  });

  state.map?.on('click', async (event) => {
    if (!state.map) {
      return;
    }

    render(event.lngLat);
  });
}

export async function render(lngLat: LngLat) {
  const bytes = await getViewshedData(lngLat);
  const segments = parseViewshedBytes(bytes);

  if (import.meta.env.DEV) {
    console.log(segments);
  }

  // longest_line.angle = longest_line.angle + ANGLE_SHIFT;
  //
  // const θ = toRadians(longest_line.angle);
  // const dx = longest_line.distance * Math.cos(θ);
  // const dy = longest_line.distance * Math.sin(θ);
  // const rotatedClockwiseAEQD = rotate(dx, dy, -0.5);
  // const rotatedAntiAEQD = rotate(dx, dy, +0.5);
  //
  // const aeqd = aeqdProjectionString(lngLat.lng, lngLat.lat);
  // const unrotated = proj4(aeqd, proj4.WGS84, [dx, dy]);
  // longest_line.from = lngLat;
  // longest_line.to = new LngLat(unrotated[0], unrotated[1]);
  // state.longestLine = longest_line;
  //
  // const rotatedClockwiseLonLat = proj4(aeqd, proj4.WGS84, rotatedClockwiseAEQD);
  // const rotatedAntiLonLat = proj4(
  //   aeqd,
  //   '+proj=longlat +datum=WGS84 +no_defs',
  //   rotatedAntiAEQD,
  // );
  // const viewCoordinates = [
  //   lngLat.toArray(),
  //   rotatedClockwiseLonLat,
  //   rotatedAntiLonLat,
  //   lngLat.toArray(),
  // ];
  //
  // const longestLineGeoJSON = {
  //   type: 'Feature',
  //   geometry: {
  //     type: 'Polygon',
  //     coordinates: [viewCoordinates],
  //   },
  //   properties: {},
  // } as GeoJSONFeature;
  //
  // const source = state.map?.getSource('longest-line') as GeoJSONSource;
  //
  // source?.setData(longestLineGeoJSON);
  //
  // state.map?.fitBounds(computeBBox(viewCoordinates), {
  //   padding: 100,
  //   duration: 1000,
  // });
  // state.isFlying = true;
  // disablePointer();
}

async function getViewshedData(lngLat: LngLat) {
  const response = await fetch(
    `http://localhost:3333/viewshed/${lngLat.lng},${lngLat.lat}`,
  );
  return await response.bytes();
}

function parseViewshedBytes(data) {
  const buffer = data.buffer || data;
  const view = new DataView(buffer);
  let offset = 0;

  const angleCount = view.getUint16(offset, false);
  console.log(angleCount);
  offset += 2;

  const out = [];

  for (let i = 0; i < angleCount; i++) {
    const byteLength = view.getUint16(offset, false);
    offset += 2;

    const values = [];
    const numElements = byteLength / 2;

    for (let j = 0; j < numElements; j++) {
      values.push(view.getUint16(offset, false));
      offset += 2;
    }

    out.push(values);
  }

  if (offset !== buffer.byteLength) {
    console.error('Viewshed unpacker did not reach end of data');
  }

  return out;
}
