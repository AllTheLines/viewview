import { PMTiles } from 'pmtiles';
import type { TileGL } from './HeatmapLayer';
import {
  MAP_SERVER,
  PMTILES_SERVER,
  tileKey,
  tileToLatLonBounds,
} from './utils';

export type WorkerEvent =
  | { type: 'init'; source: string }
  | ({ type: 'tile' } & Omit<TileGL, 'texture'> & { data: Uint8Array })
  | { type: 'getTile'; z: number; x: number; y: number };

const CACHE_BUSTER = '?buster=9/12/2025';

let localTiler: PMTiles;

const loading = new Map();

self.onmessage = async (event: MessageEvent<WorkerEvent>) => {
  if (event.data.type === 'init') {
    const { source } = event.data;
    if (import.meta.env.DEV) {
      localTiler = new PMTiles(source);
    }
    console.debug('Tile worker ready for:', source);
  }

  if (event.data.type === 'getTile') {
    const { z, x, y } = event.data;
    const key = tileKey(z, x, y);
    if (loading.get(key)) {
      return;
    } else {
      loading.set(key, true);
    }

    let bytes: Uint8Array<ArrayBufferLike> | ArrayBuffer;

    const isProductionMapServer =
      !import.meta.env.DEV || localTiler.source.getKey().includes(MAP_SERVER);
    if (isProductionMapServer) {
      const tile = await fetch(
        `${PMTILES_SERVER}/${z}/${x}/${y}.bin${CACHE_BUSTER}`,
      );
      if (tile.status === 404) {
        // This is normal. We don't have tiles that cover the sea for example.
        // TODO: also cache this so we don't keep trying to fetch tiles that don't exist.
        return;
      }
      bytes = await tile.bytes();
    } else {
      const response = await localTiler.getZxy(z, x, y);
      if (!response) return;
      bytes = response.data;
    }

    const compressed = new Uint8Array(bytes);
    const stream = new DecompressionStream('deflate');
    const decompressedResponse = new Response(
      new Blob([compressed]).stream().pipeThrough(stream),
    );
    const arrayBuffer = await decompressedResponse.arrayBuffer();

    const tvs_surfaces = new Float32Array(arrayBuffer);
    const packed = new Uint8Array(tvs_surfaces.buffer);

    // Find the greatest point of visibility. This is used to calibrate the heatmap
    // colour range for every viewport and zoom level.
    let max = -Infinity;
    for (let i = 0; i < tvs_surfaces.length; i++) {
      const value = tvs_surfaces[i];
      if (value > max) max = value;
    }

    const bounds = tileToLatLonBounds(z, x, y);

    const message: WorkerEvent = {
      type: 'tile',
      key,
      data: packed,
      max,
      bounds,
    };
    self.postMessage(message);
    loading.set(key, false);
  }
};
