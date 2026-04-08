import type { GeoJSONSource, LngLat } from 'maplibre-gl';
import { state } from '../state.svelte.ts';
import { type PolarSegments, Viewshed } from './polarSegments.ts';

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
      'fill-color': '#00ff00',
      'fill-outline-color': 'transparent',
      'fill-opacity': 0.4,
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

  const scale = 50; // TODO: Get from API?
  const builder = new Viewshed(lngLat, scale, segments);
  const viewshed = builder.create();

  const source = state.map?.getSource('viewshed') as GeoJSONSource;

  if (viewshed) {
    source?.setData(viewshed);
  }
}

async function getViewshedData(lngLat: LngLat) {
  let apiBase = 'https://api.alltheviews.world';
  if (import.meta.env.DEV) {
    apiBase = 'http://localhost:3333';
  }
  const response = await fetch(
    `${apiBase}/viewshed/${lngLat.lng},${lngLat.lat}`,
  );
  return await response.bytes();
}

function parseViewshedBytes(data: Uint8Array<ArrayBuffer>): PolarSegments[] {
  const buffer = data.buffer;
  const view = new DataView(buffer);
  let offset = 0;
  const out: PolarSegments[] = [];

  while (offset < buffer.byteLength) {
    const angleID = view.getUint16(offset, false);
    offset += 2;

    const segmentsLength = view.getUint16(offset, false);
    offset += 2;

    const pairs = [];
    const numElements = segmentsLength / 2;

    for (let j = 0; j < numElements; j++) {
      pairs.push(view.getUint16(offset, false));
      offset += 2;
    }

    out.push({ angleID, pairs: pairs });
  }

  return out;
}
