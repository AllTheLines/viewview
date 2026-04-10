import type { LngLat } from 'maplibre-gl';
import { state } from '../state.svelte.ts';
import { DEFAULT_OPACITY, type PolarSegments, Viewshed } from './Viewshed.ts';

export function setupViewsheds() {
  renderAllViewsheds();

  state.map?.on('click', async (event) => {
    if (!state.map) {
      return;
    }

    removeUnlockedViewsheds();
    renderNewViewshed(event.lngLat);
  });
}

export function renderAllViewsheds() {
  for (const viewshed of state.viewsheds) {
    const layer = state.map?.getLayer(viewshed.getViewshedLayerID());
    if (!layer) {
      addViewshed(viewshed);
    }

    state.map?.setLayoutProperty(
      viewshed.getViewshedLayerID(),
      'visibility',
      viewshed.isVisible ? 'visible' : 'none',
    );
  }
}

export function highlightViewshed(viewshedToHighlight: Viewshed) {
  for (const viewshed of state.viewsheds) {
    if (viewshed.id === viewshedToHighlight.id) continue;

    state.map?.setPaintProperty(
      viewshed.getViewshedLayerID(),
      'fill-opacity',
      0.05,
    );
  }
}

export function desaturateAllViewsheds() {
  for (const viewshed of state.viewsheds) {
    state.map?.setPaintProperty(
      viewshed.getViewshedLayerID(),
      'fill-opacity',
      DEFAULT_OPACITY,
    );
  }
}

export function updateViewshedColour(viewshed: Viewshed) {
  state.map?.setPaintProperty(
    viewshed.getViewshedLayerID(),
    'fill-color',
    viewshed.colour,
  );
}

export function removeUnlockedViewsheds() {
  for (const viewshed of state.viewsheds) {
    const layer = state.map?.getLayer(viewshed.getViewshedLayerID());

    if (layer && !viewshed.isLocked) {
      removeViewshed(viewshed);
    }
  }
}

export function removeViewshed(viewshed: Viewshed) {
  state.viewsheds = state.viewsheds.filter((v) => v !== viewshed);
  state.map?.removeLayer(viewshed.getViewshedLayerID());
  state.map?.removeSource(viewshed.getViewshedSourceID());
}

export async function renderNewViewshed(lngLat: LngLat) {
  const bytes = await getViewshedData(lngLat);

  const segments = parseViewshedBytes(bytes);

  const scale = 50; // TODO: Get from API?
  const id = crypto.randomUUID();

  const viewshed = new Viewshed(id, lngLat, scale, segments);
  state.viewsheds.push(viewshed);

  renderAllViewsheds();
}

function addViewshed(viewshed: Viewshed) {
  const sourceID = viewshed.getViewshedSourceID();

  state.map?.addSource(sourceID, {
    type: 'geojson',
    data: viewshed.geoJSON(),
  });

  state.map?.addLayer({
    id: viewshed.getViewshedLayerID(),
    type: 'fill',
    source: sourceID,
    paint: {
      'fill-color': viewshed.colour,
      'fill-outline-color': 'transparent',
      'fill-opacity': DEFAULT_OPACITY,
    },
  });
}

async function getViewshedData(lngLat: LngLat) {
  let apiBase = 'https://api.alltheviews.world';
  if (import.meta.env.DEV) {
    apiBase = 'http://localhost:3333';
  }

  const start = performance.now();
  const response = await fetch(
    `${apiBase}/viewshed/${lngLat.lng},${lngLat.lat}`,
  );
  const end = performance.now();
  if (import.meta.env.DEV) {
    console.log(`Viewshed fetched in ms`, end - start);
  }

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
