import type {
  CustomLayerInterface,
  LngLatBounds,
  Map as MapLibre,
} from 'maplibre-gl';
import fragment from './fragment.glsl?raw';
import {
  BUCKET,
  getParentTile,
  isTileIntersectingBounds,
  tileKey,
} from './utils';
import vertex from './vertex.glsl?raw';
import type { WorkerEvent } from './Worker';

export type TileGL = {
  key: string;
  texture: WebGLTexture;
  min: number;
  max: number;
  bounds: LngLatBounds;
};

type Uniforms = {
  uProjectionMatrix: WebGLUniformLocation | null;
  uTileMatrix: WebGLUniformLocation | null;
  uData: WebGLUniformLocation | null;
  uMax: WebGLUniformLocation | null;
  uScale: WebGLUniformLocation | null;
  uOffset: WebGLUniformLocation | null;
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

let state: State;

function initialise() {
  if (state === undefined) {
    return;
  }

  const PMTILES = `${BUCKET}/pmtiles`;
  const WORLD_PMTILES = 'world.pmtiles';

  const params = new URLSearchParams(self.location.search);
  let source = params.get('pmtiles');
  if (!source) {
    if (import.meta.env.DEV) {
      source = `/${WORLD_PMTILES}`;
    } else {
      source = `${PMTILES}/${WORLD_PMTILES}`;
    }
  }
  state.worker.postMessage({ type: 'init', source });
  state.worker.onmessage = onWorkerMessage;
}

function onWorkerMessage(event: MessageEvent<WorkerEvent>) {
  if (state === undefined) {
    return;
  }

  if (event.data.type === 'tile') {
    const { key, data, min, max, bounds } = event.data;
    const texture = state.gl.createTexture();
    state.gl.bindTexture(state.gl.TEXTURE_2D, texture);
    state.gl.texParameteri(
      state.gl.TEXTURE_2D,
      state.gl.TEXTURE_MIN_FILTER,
      state.gl.LINEAR,
    );
    state.gl.texParameteri(
      state.gl.TEXTURE_2D,
      state.gl.TEXTURE_MAG_FILTER,
      state.gl.LINEAR,
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
      state.gl.R32F,
      config.tileSize,
      config.tileSize,
      0,
      state.gl.RED,
      state.gl.FLOAT,
      data,
    );

    const tile: TileGL = {
      key,
      bounds,
      min,
      max,
      texture,
    };

    state.tileCache.set(key, tile);
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

    gl.getExtension('EXT_color_buffer_float');
    gl.getExtension('OES_texture_float_linear');

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
      uData: gl.getUniformLocation(program, 'u_data'),
      uMax: gl.getUniformLocation(program, 'u_max'),
      uScale: gl.getUniformLocation(program, 'u_scale'),
      uOffset: gl.getUniformLocation(program, 'u_offset'),
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

    if (Date.now() - state.lastGC < 1000) {
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
        continue;
      }

      if (cachedTile.max > max) {
        max = cachedTile.max;
      }
    }

    gl.useProgram(state.program);
    gl.bindBuffer(gl.ARRAY_BUFFER, state.vertexBuffer);
    const positionLocation = gl.getAttribLocation(state.program, 'a_pos');
    gl.enableVertexAttribArray(positionLocation);
    gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

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
        continue;
      }

      const projection = state.map.transform.getProjectionData({
        overscaledTileID: tile,
      });

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, cachedTile.texture);
      gl.uniform1i(state.uniforms.uData, 0);

      gl.uniformMatrix4fv(
        state.uniforms.uProjectionMatrix,
        false,
        new Float32Array(matrix.defaultProjectionData.mainMatrix),
      );
      gl.uniform4f(
        state.uniforms.uTileMatrix,
        ...projection.tileMercatorCoords,
      );
      gl.uniform1f(state.uniforms.uMax, max);
      gl.uniform1f(state.uniforms.uScale, scaleIfParent);
      gl.uniform2f(state.uniforms.uOffset, offsetIfParentX, offsetIfParentY);

      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
  },
};

export { HeatmapLayer };
