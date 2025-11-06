import { PMTiles } from 'pmtiles';
import type { TileGL } from './HeatmapLayer';
import { tileKey, tileToLatLonBounds } from './utils';

export type WorkerEvent =
  | { type: 'init'; source: string }
  | ({ type: 'tile' } & Omit<TileGL, 'texture'> & { data: Float32Array })
  | { type: 'getTile'; z: number; x: number; y: number };

let heatmapTiles: PMTiles;

const loading = new Map();

self.onmessage = async (event: MessageEvent<WorkerEvent>) => {
  if (event.data.type === 'init') {
    const { source } = event.data;
    heatmapTiles = new PMTiles(source);
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

    const tile = await heatmapTiles.getZxy(z, x, y);
    if (!tile) {
      // This is normal. We don't have tiles that cover the sea for example.
      // TODO: also cache this so we don't keep trying to fetch tiles that don't exist.
      return;
    }

    const compressed = new Uint8Array(tile.data);
    const stream = new DecompressionStream('deflate');
    const decompressedResp = new Response(
      new Blob([compressed]).stream().pipeThrough(stream),
    );
    const arrayBuffer = await decompressedResp.arrayBuffer();

    const tvs_surfaces = new Float32Array(arrayBuffer);
    const min = Math.min(...tvs_surfaces);
    const max = Math.max(...tvs_surfaces);
    const bounds = tileToLatLonBounds(z, x, y);

    const message: WorkerEvent = {
      type: 'tile',
      key,
      data: tvs_surfaces,
      min,
      max,
      bounds,
    };
    self.postMessage(message);
    loading.set(key, false);
  }
};
