import { LngLat } from 'maplibre-gl';
import { type PolarSegments, Viewshed } from './lib/Viewshed';

export type ViewshedWorkerEvent =
  | { type: 'getViewshed'; coordinate: LngLat }
  | { type: 'setViewshed'; viewshed: Viewshed }
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
      type: 'setViewshed',
      viewshed,
    } as ViewshedWorkerEvent;
    self.postMessage(messageDirty);

    // TODO:
    //   This is reeeeeally slow for higher angular resolutions.
    //   So we need to construct the viewsheds server side
    //
    // viewshed.unionGeoJSON();
    // const messageClean = {
    //   type: "updateViewshed",
    //   viewshed,
    // } as ViewshedWorkerEvent;
    // self.postMessage(messageClean);
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

    const pairs = [];
    const numElements = segmentsLength / 2;

    for (let j = 0; j < numElements; j++) {
      pairs.push(view.getUint16(offset, false));
      offset += 2;
    }

    segments.push({ angle, pairs: pairs });
  }

  return { angleScale, lonLatOfBiggestViewshed: lngLat, segments };
}
