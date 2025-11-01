let tileCache = new Map();
const worker = new Worker(new URL('./worker.js', import.meta.url));

worker.onmessage = (event) => {
	const { type, key, data, min, max } = event.data;

	if (type === "tile") {
		const tile = {
			data,
			min,
			max,
		}

		tileCache.set(key, tile);
		console.debug(`Tile loaded: ${key}`);
	}
};

const params = new URLSearchParams(self.location.search);
const source = params.get('source');
worker.postMessage({ type: "init", source });

function tileKey(z, x, y) {
	return `${z}/${x}/${y}`;
}

const map = new maplibregl.Map({
	container: "map",
	zoom: 1,
	center: [-3, 53],
	style: {
		version: 8,
		sources: {
			osm: {
				type: 'raster',
				tiles: [
					'https://tile.openstreetmap.org/{z}/{x}/{y}.png'
				],
				tileSize: 256,
				attribution:
					'© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
			}
		},
		layers: [
			{
				id: 'osm-layer',
				type: 'raster',
				source: 'osm',
				minzoom: 0,
				maxzoom: 14
			}
		]
	}
});

const HeatmapLayer = {
	id: "heatmap-tiles",
	type: "custom",
	renderingMode: "2d",

	async onAdd(map, gl) {
		this.gl = gl;
		this.map = map;
		this.tiles = new Map();
		this.tileSize = 256;

		if (!(gl instanceof WebGL2RenderingContext)) {
			console.error("Need WebGL2 for R32F textures.");
		}

		gl.getExtension('EXT_color_buffer_float');
		gl.getExtension('OES_texture_float_linear');

		const vs = `
		    precision highp float;
		    attribute vec2 a_pos;
		    uniform mat4 u_projectionMatrix;
		    uniform vec4 u_tileMatrix;
		    varying vec2 v_texcoord;
		    void main() {
		      v_texcoord = (a_pos + 1.0) / 2.0;
					  
					// Why??
					float magicScaler = 2.0;
					
					vec2 tileOrigin = u_tileMatrix.xy;
					vec2 tileSize = u_tileMatrix.zw * magicScaler;
					vec2 in_tile = a_pos;
					vec4 worldPosition = vec4(tileOrigin + in_tile * tileSize, 0.0, 1.0);

					gl_Position = u_projectionMatrix * worldPosition;
		    }
		  `;

		const fs = `
		    precision highp float;
		    varying vec2 v_texcoord;
		    uniform sampler2D u_data;
		    uniform float u_max;
		    void main() {
		      vec2 coord = v_texcoord / 2048.0;
		      float value = texture2D(u_data, coord).r;

					float normalized = value / u_max;
					float normalized_v = pow(normalized, 0.5);
					
					vec3 color_min = vec3(0.0, 0.0, 0.0);
					vec3 color_mid = vec3(0.5, 0.5, 0.5);
					vec3 color_max = vec3(1.0, 1.0, 1.0);

					vec3 final_color;

					if (normalized_v < 0.5) {
							float half_normalized = normalized_v / 0.5;
							final_color = mix(color_min, color_mid, half_normalized);
					} else {
							float half_normalized = (normalized_v - 0.5) / 0.5;
							final_color = mix(color_mid, color_max, half_normalized);
					}


					float alpha = 1.0;
					if (normalized <= 0.0) {
            alpha = 0.0;
					}
		      gl_FragColor = vec4(final_color, alpha);
		    }
		  `;

		const compile = (src, type) => {
			const shader = gl.createShader(type);
			gl.shaderSource(shader, src);
			gl.compileShader(shader);
			if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
				throw new Error(gl.getShaderInfoLog(shader));
			return shader;
		};

		const program = gl.createProgram();
		gl.attachShader(program, compile(vs, gl.VERTEX_SHADER));
		gl.attachShader(program, compile(fs, gl.FRAGMENT_SHADER));
		gl.linkProgram(program);
		this.program = program;

		this.vertexBuffer = gl.createBuffer();
		gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer);
		gl.bufferData(
			gl.ARRAY_BUFFER,
			new Float32Array([0, 0, 4096, 0, 0, 4096, 4096, 4096]),
			gl.STATIC_DRAW
		);

		this.uProjectionMatrix = gl.getUniformLocation(program, "u_projectionMatrix");
		this.uTileMatrix = gl.getUniformLocation(program, "u_tileMatrix");
		this.uData = gl.getUniformLocation(program, "u_data");
		this.uColormap = gl.getUniformLocation(program, "u_colormap");
		this.uMax = gl.getUniformLocation(program, "u_max");
	},

	prerender() {
		// TODO: Delete unused cached tiles.
	},

	async render(gl, matrix) {
		let max = 0.0;

		for (const tile of map.coveringTiles({ tileSize: 256 })) {
			let key = tileKey(tile.canonical.z, tile.canonical.x, tile.canonical.y);
			let cachedTile = tileCache.get(key);
			if (!cachedTile) {
				worker.postMessage({
					type: "getTile",
					z: tile.canonical.z,
					x: tile.canonical.x,
					y: tile.canonical.y
				});
				continue;
			};
			if (cachedTile.max > max) {
				max = cachedTile.max;
			}
		};

		gl.useProgram(this.program);
		gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer);
		const posLoc = gl.getAttribLocation(this.program, "a_pos");
		gl.enableVertexAttribArray(posLoc);
		gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
		gl.enable(gl.BLEND);
		gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

		for (const tile of map.coveringTiles({ tileSize: 256 })) {
			let key = tileKey(tile.canonical.z, tile.canonical.x, tile.canonical.y);
			let cachedTile = tileCache.get(key);
			if (!cachedTile) {
				continue;
			};

			if (!cachedTile.texture) {
				const texture = gl.createTexture();
				gl.bindTexture(gl.TEXTURE_2D, texture);
				gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
				gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
				gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
				gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
				gl.texImage2D(
					gl.TEXTURE_2D,
					0,
					gl.R32F,
					this.tileSize,
					this.tileSize,
					0,
					gl.RED,
					gl.FLOAT,
					cachedTile.data
				);
				cachedTile.texture = texture;
				tileCache.set(key, cachedTile);
			}

			const projection = map.transform.getProjectionData({
				overscaledTileID: tile,
			});

			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D, cachedTile.texture);
			gl.uniform1i(this.uData, 0);

			gl.uniform1f(this.uMax, max);
			gl.uniformMatrix4fv(this.uProjectionMatrix, false, matrix.defaultProjectionData.mainMatrix);
			gl.uniform4f(this.uTileMatrix, ...projection.tileMercatorCoords);

			gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
		}
	},
};

map.on("load", () => {
	map.addLayer(HeatmapLayer);
});
