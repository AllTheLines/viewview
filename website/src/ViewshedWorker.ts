import { LngLat } from 'maplibre-gl';
import { type PolarSegments, Viewshed } from './lib/Viewshed';
import init from './lib/viewshed-reconstructor/viewshed_reconstructor.js';

await init();
self.postMessage({ status: 'ready' });

export type ViewshedWorkerEvent =
  | { type: 'getViewshed'; coordinate: LngLat }
  | { type: 'renderViewshed'; viewshed: Viewshed }
  | { type: 'updateViewshed'; viewshed: Viewshed };

self.onmessage = async (event: MessageEvent<ViewshedWorkerEvent>) => {
  if (event.data.type === 'getViewshed') {
    const bytes = await getViewshedData(event.data.coordinate);

    const payload = parseViewshedBytes(bytes);

    let demScale = 1; // TODO: Get from API?
    if (import.meta.env.DEV) {
      demScale = 100;
    }

    const id = crypto.randomUUID();

    const viewshed = new Viewshed(
      id,
      payload.lonLatOfBiggestViewshed,
      demScale,
      payload.angleScale,
      payload.segments,
    );

    const messageDirty = {
      type: 'renderViewshed',
      viewshed,
    } as ViewshedWorkerEvent;
    self.postMessage(messageDirty);
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
    console.log(`Viewshed fetched in: ${end - start}ms`);
  }

  return await response.bytes();
}

function parseViewshedBytes(data: Uint8Array<ArrayBuffer>): {
  angleScale: number;
  lonLatOfBiggestViewshed: LngLat;
  segments: PolarSegments[];
} {
  const buffer = data.buffer;
  const view = new DataView(buffer);
  let offset = 0;
  const segments: PolarSegments[] = [];

  const angleScale = view.getUint32(offset);
  offset += 4;

  const lonOfBiggestViewshed = view.getFloat32(offset, false);
  offset += 4;
  const latOfBiggestViewshed = view.getFloat32(offset, false);
  offset += 4;
  const lngLat = new LngLat(lonOfBiggestViewshed, latOfBiggestViewshed);

  while (offset < buffer.byteLength) {
    const angle = view.getUint16(offset, false) / angleScale;
    offset += 2;

    const segmentsLength = view.getUint16(offset, false);
    offset += 2;

    const bitpacks = [];
    const numElements = segmentsLength / 4;

    for (let i = 0; i < numElements; i++) {
      bitpacks.push(view.getUint32(offset, false));
      offset += 4;
    }

    segments.push({ angle, bitpacks });
  }

  return { angleScale, lonLatOfBiggestViewshed: lngLat, segments };
}
