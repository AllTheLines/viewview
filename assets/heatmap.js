const params = new URLSearchParams(window.location.search);
const source = params.get('source');

let protocol = new pmtiles.Protocol({ metadata: true });
maplibregl.addProtocol("pmtiles", protocol.tile);
const heatmapTiles = new pmtiles.PMTiles(source);
protocol.add(heatmapTiles);

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
		this.tileSize = 512;

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

	async loadTile(z, x, y) {
		const key = `${z}/${x}/${y}`;
		if (this.tiles.has(key)) { return false };

		try {
			var data = { isLoaded: false };
			this.tiles.set(key, data);
			const tileBuf = await heatmapTiles.getZxy(z, x, y);
			if (!tileBuf) return null;

			const tvs_surfaces = new Float32Array(tileBuf.data);
			const minVal = Math.min(...tvs_surfaces);
			const maxVal = Math.max(...tvs_surfaces);

			const tex = this.gl.createTexture();
			this.gl.bindTexture(this.gl.TEXTURE_2D, tex);
			this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MIN_FILTER, this.gl.LINEAR);
			this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_MAG_FILTER, this.gl.LINEAR);
			this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_S, this.gl.CLAMP_TO_EDGE);
			this.gl.texParameteri(this.gl.TEXTURE_2D, this.gl.TEXTURE_WRAP_T, this.gl.CLAMP_TO_EDGE);
			this.gl.texImage2D(
				this.gl.TEXTURE_2D,
				0,
				this.gl.R32F,
				this.tileSize,
				this.tileSize,
				0,
				this.gl.RED,
				this.gl.FLOAT,
				tvs_surfaces
			);

			data = { tex, minVal, maxVal, isLoaded: true };
			this.tiles.set(key, data);
			console.debug(`Tile loaded: ${key}`);
			return true;
		} catch (e) {
			console.error("Tile fetch error:", e);
			return null;
		}
	},

	prerender() {
		// for (const tile of map.coveringTiles({ tileSize: 256 })) {
		// 	const key = `${tile.canonical.z}/${tile.canonical.x}/${tile.canonical.y}`;
		// };
		// // Get the list of currently visible tile IDs
		// const visibleTileKeys = new Set(this.map.getVisibleTiles().map(this.tileManager.getTileKey));
		//
		// // Check for any tiles in the cache that are no longer visible
		// for (const key of this.tiles.keys()) {
		// 	if (!visibleTileKeys.has(key)) {
		// 		const texture = this.tiles.get(key);
		// 		gl.deleteTexture(texture); // Delete the GL texture
		// 		this.tiles.delete(key);    // Remove from the cache
		// 	}
		// }
	},

	async render(gl, matrix) {
		let loading = false;
		let max = 0.0;

		// We do this is in a seperate loop because:
		// * You can't call await during GL setup.
		// * We need to calculate the max total surface accumulation for the entire viewport.
		for (const tile of map.coveringTiles({ tileSize: 256 })) {
			loading = await this.loadTile(tile.canonical.z, tile.canonical.x, tile.canonical.y);
			const key = `${tile.canonical.z}/${tile.canonical.x}/${tile.canonical.y}`;
			let cachedTile = this.tiles.get(key);
			if (cachedTile.maxVal > max) {
				max = cachedTile.maxVal;
			}

		};

		if (loading) {
			this.map.triggerRepaint();
			return;
		};

		gl.useProgram(this.program);
		gl.bindBuffer(gl.ARRAY_BUFFER, this.vertexBuffer);
		const posLoc = gl.getAttribLocation(this.program, "a_pos");
		gl.enableVertexAttribArray(posLoc);
		gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
		gl.enable(gl.BLEND);
		gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

		for (const tile of map.coveringTiles({ tileSize: 256 })) {
			const key = `${tile.canonical.z}/${tile.canonical.x}/${tile.canonical.y}`
			let cachedTile = this.tiles.get(key);
			if (!cachedTile.isLoaded) continue;
			console.debug(`Rendering: ${key}`);

			const projection = map.transform.getProjectionData({
				overscaledTileID: tile,
			});

			gl.activeTexture(gl.TEXTURE0);
			gl.bindTexture(gl.TEXTURE_2D, cachedTile.tex);
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
