import { PMTiles } from 'pmtiles';
import type { TileGL } from './HeatmapLayer';
import {
  CACHE_BUSTER,
  MAP_SERVER,
  PMTILES_SERVER,
  tileKey,
  tileToLatLonBounds,
} from './utils';

export type WorkerEvent =
  | { type: 'init'; source: string }
  | ({ type: 'tile' } & Omit<TileGL, 'texture'> & { data: Uint8Array })
  | { type: 'getTile'; z: number; x: number; y: number };

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
    const url = `${PMTILES_SERVER}/${z}/${x}/${y}.bin${CACHE_BUSTER}`;

    const isProductionMapServer =
      !import.meta.env.DEV || localTiler.source.getKey().includes(MAP_SERVER);
    if (isProductionMapServer) {
      let tile: Response;
      tile = await fetch(url);
      if (tile.status === 204 || tile.status === 404) {
        // This is normal. We don't have tiles that cover the sea for example.
        // TODO: also cache this so we don't keep trying to fetch tiles that don't exist.
        return;
      }

      if (!tile.ok) {
        console.warn(`Bad tile response ${tile.status} for ${url}`);
        return;
      }

      bytes = await tile.bytes();

      if (bytes.length === 0) {
        return;
      }
    } else {
      const response = await localTiler.getZxy(z, x, y);
      if (!response) return;
      bytes = response.data;
    }

    let tvs_surfaces: Float32Array;
    const compressed = new Uint8Array(bytes);
    const stream = new DecompressionStream('deflate');
    const decompressedResponse = new Response(
      new Blob([compressed]).stream().pipeThrough(stream),
    );

    try {
      const arrayBuffer = await decompressedResponse.arrayBuffer();
      tvs_surfaces = new Float32Array(arrayBuffer);
    } catch (error) {
      console.error('Decompression failed for', url, error);
      return;
    }

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
      data: new Uint8Array(tvs_surfaces.buffer),
      max,
      bounds,
    };
    self.postMessage(message);
    loading.set(key, false);
  }
};
