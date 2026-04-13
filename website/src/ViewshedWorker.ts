import type { LngLat } from 'maplibre-gl';
import { type PolarSegments, Viewshed } from './lib/Viewshed';

export type ViewshedWorkerEvent =
  | { type: 'getViewshed'; coordinate: LngLat }
  | { type: 'setViewshed'; viewshed: Viewshed }
  | { type: 'updateViewshed'; viewshed: Viewshed };

self.onmessage = async (event: MessageEvent<ViewshedWorkerEvent>) => {
  if (event.data.type === 'getViewshed') {
    const bytes = await getViewshedData(event.data.coordinate);

    const segments = parseViewshedBytes(bytes);

    const scale = 50; // TODO: Get from API?
    const id = crypto.randomUUID();

    const viewshed = new Viewshed(id, event.data.coordinate, scale, segments);
    const messageDirty = {
      type: 'setViewshed',
      viewshed,
    } as ViewshedWorkerEvent;
    self.postMessage(messageDirty);

    viewshed.unionGeoJSON();
    const messageClean = {
      type: 'updateViewshed',
      viewshed,
    } as ViewshedWorkerEvent;
    self.postMessage(messageClean);
  }
};

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
