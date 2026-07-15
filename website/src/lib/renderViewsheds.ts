import { state } from '../state.svelte.ts';
import type { ViewshedWorkerEvent } from '../ViewshedWorker.ts';
import {
  endLoadingSpinner,
  extractCoordFromURL,
  startLoadingSpinner,
} from './utils.ts';
import { DEFAULT_OPACITY, Viewshed } from './Viewshed.ts';

function startViewshedWorker(): Promise<Worker> {
  return new Promise((resolve) => {
    const worker = new Worker(new URL('../ViewshedWorker.js', import.meta.url));
    worker.onmessage = mainThreadViewshedWorkerCallbacks;

    worker.addEventListener('message', function onReady(event) {
      if (event.data.status === 'ready') {
        worker.removeEventListener('message', onReady);
        resolve(worker);
      }
    });
  });
}

export async function setupViewsheds(coordFromURL: string | undefined) {
  const worker = await startViewshedWorker();

  renderAllViewsheds();

  if (coordFromURL?.startsWith('viewshed')) {
    const coordinate = extractCoordFromURL(
      coordFromURL.replace('viewshed/', ''),
    );
    startLoadingSpinner();
    worker.postMessage({
      type: 'getViewshed',
      coordinate,
    } as ViewshedWorkerEvent);
  }

  state.map?.on('click', async (event) => {
    if (!state.map) {
      return;
    }

    removeUnlockedViewsheds();

    startLoadingSpinner();
    worker.postMessage({
      type: 'getViewshed',
      coordinate: event.lngLat,
    } as ViewshedWorkerEvent);
  });
}

export function mainThreadViewshedWorkerCallbacks(
  event: MessageEvent<ViewshedWorkerEvent>,
) {
  const isViewshedData =
    event.data.type === 'renderViewshed' ||
    event.data.type === 'updateViewshed';
  if (!isViewshedData) return;

  const viewshed = event.data.viewshed;
  Object.setPrototypeOf(viewshed, Viewshed.prototype); // Rehydrate the serialised data.

  if (event.data.type === 'renderViewshed') {
    state.viewsheds.push(viewshed);
    renderAllViewsheds();
  }

  if (event.data.type === 'updateViewshed') {
    updateExistingViewshedData(viewshed);
  }

  endLoadingSpinner();
}

function updateExistingViewshedData(viewshed: Viewshed) {
  const source = state.map?.getSource(viewshed.getViewshedSourceID());
  // @ts-expect-error: For some reason TS thinks this is for a tile layer.
  source?.setData(viewshed.geoJSON);
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

function addViewshed(viewshed: Viewshed) {
  const sourceID = viewshed.getViewshedSourceID();

  state.map?.addSource(sourceID, {
    type: 'geojson',
    data: viewshed.geoJSON,
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
