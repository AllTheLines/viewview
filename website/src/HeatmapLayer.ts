import {
  type CustomLayerInterface,
  LngLatBounds,
  type Map as MapLibre,
} from 'maplibre-gl';
import fragment from './fragment.glsl?raw';
import { state as sharedState } from './state.svelte.ts';
import {
  getParentTile,
  isTileIntersectingBounds,
  Log,
  PMTILES_SERVER,
  packFloatToU8s,
  tileKey,
} from './utils';
import vertex from './vertex.glsl?raw';
import type { WorkerEvent } from './Worker';

export type TileGL = {
  key: string;
  texture: WebGLTexture;
  max: number;
  bounds: LngLatBounds;
};

type Uniforms = {
  uProjectionMatrix: WebGLUniformLocation | null;
  uTileMatrix: WebGLUniformLocation | null;
  uWorldOffset: WebGLUniformLocation | null;
  uData: WebGLUniformLocation | null;
  uContrast: WebGLUniformLocation | null;
  uIntensity: WebGLUniformLocation | null;
  uMax: WebGLUniformLocation | null;
  uScale: WebGLUniformLocation | null;
  uOffset: WebGLUniformLocation | null;
  uAverageSurfaceVisibility: WebGLUniformLocation | null;
};

type State =
  | {
      map: MapLibre;
      gl: WebGL2RenderingContext;
      program: WebGLProgram;
      vertexBuffer: WebGLBuffer;
      tileCache: Map<string, TileGL>;
      uniforms: Uniforms;
      worker: Worker;
      lastGC: number;
    }
  | undefined;

const config: { tileSize: number } = {
  tileSize: 256,
};

// The average surface area visibile from a point far out at sea, where it can only see sea.
// This is used to fill regions for which there is no elevation data.
const AVERAGE_SURFACE_VISIBILITY = 700000.0;

let fillerTile: TileGL;

let state: State;

function initialise() {
  if (state === undefined) {
    return;
  }

  const params = new URLSearchParams(self.location.search);
  let source = params.get('pmtiles');
  if (!source) {
    source = PMTILES_SERVER;
  }
  state.worker.postMessage({ type: 'init', source });
  state.worker.onmessage = onWorkerMessage;

  makeFillerTile();
}

function onWorkerMessage(event: MessageEvent<WorkerEvent>) {
  if (state === undefined) {
    return;
  }

  if (event.data.type === 'tile') {
    const { key, data, max, bounds } = event.data;
    const tile = makeTile(key, max, bounds, data);
    if (tile === undefined) {
      return;
    }

    state.tileCache.set(key, tile);

    // Should these be throttled?
    state.map?.redraw();
  }
}

const HeatmapLayer: CustomLayerInterface = {
  id: 'heatmap-tiles',
  type: 'custom',
  renderingMode: '2d',

  async onAdd(map: MapLibre, gl: WebGL2RenderingContext) {
    if (!(gl instanceof WebGL2RenderingContext)) {
      console.error('Need WebGL2 for R32F textures.');
    }

    const compile = (source: string, type: GLenum) => {
      const shader = gl.createShader(type);
      if (shader == null) {
        throw Error(`Couldn't create shader ${source}`);
      }
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        const log = gl.getShaderInfoLog(shader);
        throw new Error(`Couldn't compile shader: ${log}`);
      }
      return shader;
    };

    const program = gl.createProgram();
    gl.attachShader(program, compile(vertex, gl.VERTEX_SHADER));
    gl.attachShader(program, compile(fragment, gl.FRAGMENT_SHADER));
    gl.linkProgram(program);

    const vertexBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([0, 0, 4096, 0, 0, 4096, 4096, 4096]),
      gl.STATIC_DRAW,
    );

    const uniforms = {
      uProjectionMatrix: gl.getUniformLocation(program, 'u_projectionMatrix'),
      uTileMatrix: gl.getUniformLocation(program, 'u_tileMatrix'),
      uWorldOffset: gl.getUniformLocation(program, 'u_worldOffset'),
      uData: gl.getUniformLocation(program, 'u_data'),
      uIntensity: gl.getUniformLocation(program, 'u_intensity'),
      uContrast: gl.getUniformLocation(program, 'u_contrast'),
      uMax: gl.getUniformLocation(program, 'u_max'),
      uScale: gl.getUniformLocation(program, 'u_scale'),
      uOffset: gl.getUniformLocation(program, 'u_offset'),
      uAverageSurfaceVisibility: gl.getUniformLocation(
        program,
        'u_averageSurfaceVisibility',
      ),
    };

    state = {
      map,
      gl,
      program,
      vertexBuffer,
      uniforms,
      tileCache: new Map(),
      worker: new Worker(new URL('./Worker.js', import.meta.url)),
      lastGC: Date.now(),
    };

    initialise();
  },

  prerender() {
    if (state === undefined) {
      return;
    }

    if (Date.now() - state.lastGC < 60 * 1000) {
      return;
    }

    const mapBounds = state.map.getBounds();

    for (const [key, tile] of state.tileCache.entries()) {
      if (isTileIntersectingBounds(tile.bounds, mapBounds)) {
      } else {
        state.tileCache.delete(key);
      }
    }

    state.lastGC = Date.now();
  },

  async render(gl, matrix) {
    let max = 0.0;

    if (state === undefined) {
      return;
    }

    let isSomethingToRender = false;
    for (const tile of state.map.coveringTiles({ tileSize: 256 })) {
      const key = tileKey(tile.canonical.z, tile.canonical.x, tile.canonical.y);
      let cachedTile = state.tileCache.get(key);
      if (!cachedTile) {
        state.worker.postMessage({
          type: 'getTile',
          z: tile.canonical.z,
          x: tile.canonical.x,
          y: tile.canonical.y,
        });

        let child = {
          z: tile.canonical.z,
          x: tile.canonical.x,
          y: tile.canonical.y,
        };
        for (let _i = tile.canonical.z; _i > 0; _i--) {
          const parent = getParentTile(child.z, child.x, child.y);
          if (parent == null) {
            continue;
          }
          const parentKey = tileKey(parent.z, parent.x, parent.y);
          cachedTile = state.tileCache.get(parentKey);

          if (cachedTile) {
            break;
          }
          child = parent;
        }
      }

      if (!cachedTile) {
        cachedTile = fillerTile;
      } else {
        isSomethingToRender = true;
      }

      if (cachedTile.max > max) {
        max = cachedTile.max;
      }
    }

    if (!isSomethingToRender) {
      // Don't render if all we have is filler tiles. They flash bang white.
      return;
    }

    gl.useProgram(state.program);
    gl.bindBuffer(gl.ARRAY_BUFFER, state.vertexBuffer);
    const positionLocation = gl.getAttribLocation(state.program, 'a_pos');
    gl.enableVertexAttribArray(positionLocation);
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);

    for (const tile of state.map.coveringTiles({ tileSize: 256 })) {
      let scaleIfParent = 1.0;
      let offsetIfParentX = 0.0;
      let offsetIfParentY = 0.0;

      const key = tileKey(tile.canonical.z, tile.canonical.x, tile.canonical.y);
      let cachedTile = state.tileCache.get(key);

      if (!cachedTile) {
        let child = {
          z: tile.canonical.z,
          x: tile.canonical.x,
          y: tile.canonical.y,
        };
        for (let _i = tile.canonical.z; _i > 0; _i--) {
          const parent = getParentTile(child.z, child.x, child.y);
          if (parent == null) {
            continue;
          }
          const parentKey = tileKey(parent.z, parent.x, parent.y);
          cachedTile = state.tileCache.get(parentKey);

          if (cachedTile) {
            const zoomDifference = tile.canonical.z - parent.z;
            scaleIfParent = 2 ** zoomDifference;
            offsetIfParentX = tile.canonical.x / scaleIfParent - parent.x;
            offsetIfParentY = tile.canonical.y / scaleIfParent - parent.y;
            break;
          }

          child = parent;
        }
      }

      if (!cachedTile) {
        cachedTile = fillerTile;
      }

      const projection = state.map.transform.getProjectionData({
        overscaledTileID: tile,
      });

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, cachedTile.texture);
      gl.uniform1i(state.uniforms.uData, 0);
      gl.uniform1f(
        state.uniforms.uIntensity,
        sharedState.heatmapConfig.intensity,
      );
      gl.uniform1f(
        state.uniforms.uContrast,
        sharedState.heatmapConfig.contrast,
      );

      gl.uniformMatrix4fv(
        state.uniforms.uProjectionMatrix,
        false,
        new Float32Array(matrix.defaultProjectionData.mainMatrix),
      );
      gl.uniform4f(
        state.uniforms.uTileMatrix,
        ...projection.tileMercatorCoords,
      );
      gl.uniform1f(state.uniforms.uWorldOffset, tile.wrap);

      gl.uniform1f(state.uniforms.uMax, max);
      gl.uniform1f(state.uniforms.uScale, scaleIfParent);
      gl.uniform2f(state.uniforms.uOffset, offsetIfParentX, offsetIfParentY);
      gl.uniform1f(
        state.uniforms.uAverageSurfaceVisibility,
        AVERAGE_SURFACE_VISIBILITY,
      );

      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
  },
};

function makeTile(
  key: string,
  max: number,
  bounds: LngLatBounds,
  data: Uint8Array,
) {
  if (state?.gl === undefined) {
    console.warn("No GL context, couldn't make tile");
    return;
  }

  const texture = state.gl.createTexture();
  state.gl.bindTexture(state.gl.TEXTURE_2D, texture);
  state.gl.texParameteri(
    state.gl.TEXTURE_2D,
    state.gl.TEXTURE_MIN_FILTER,
    state.gl.NEAREST,
  );
  state.gl.texParameteri(
    state.gl.TEXTURE_2D,
    state.gl.TEXTURE_MAG_FILTER,
    state.gl.NEAREST,
  );
  state.gl.texParameteri(
    state.gl.TEXTURE_2D,
    state.gl.TEXTURE_WRAP_S,
    state.gl.CLAMP_TO_EDGE,
  );
  state.gl.texParameteri(
    state.gl.TEXTURE_2D,
    state.gl.TEXTURE_WRAP_T,
    state.gl.CLAMP_TO_EDGE,
  );
  state.gl.texImage2D(
    state.gl.TEXTURE_2D,
    0,
    state.gl.RGBA8UI,
    config.tileSize,
    config.tileSize,
    0,
    state.gl.RGBA_INTEGER,
    state.gl.UNSIGNED_BYTE,
    data,
  );

  return {
    key,
    bounds,
    max,
    texture,
  } as TileGL;
}

function makeFillerTile() {
  const data = new Uint8Array(config.tileSize ** 2 * 4);
  data.set(packFloatToU8s(AVERAGE_SURFACE_VISIBILITY));

  const tile = makeTile(
    'filler',
    AVERAGE_SURFACE_VISIBILITY,
    new LngLatBounds(),
    data,
  );

  if (tile === undefined) {
    return;
  }

  Log.debug('Filler tile created');

  fillerTile = tile;
}

export { HeatmapLayer };
