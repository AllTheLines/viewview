importScripts("https://unpkg.com/pmtiles@4.3.0/dist/pmtiles.js");

let heatmapTiles;

let loading = new Map();

self.onmessage = async (event) => {
	const { type, ...args } = event.data;

	if (type === "init") {
		const { source } = args;
		heatmapTiles = new pmtiles.PMTiles(source);
		console.debug("Tile worker ready for:", source);
	}

	if (type === "getTile") {
		const { z, x, y } = args;
		let key = tileKey(z, x, y);
		if (loading.get(key)) {
			return;
		} else {
			loading.set(key, true);
		}

		const tile = await heatmapTiles.getZxy(z, x, y);
		if (!tile) {
			/// This is normal. We don't have tiles that cover the sea for example.
			return;
		}

		const compressed = new Uint8Array(tile.data);
		const stream = new DecompressionStream("deflate");
		const decompressedResp = new Response(
			new Blob([compressed]).stream().pipeThrough(stream)
		);
		const arrayBuffer = await decompressedResp.arrayBuffer();

		const tvs_surfaces = new Float32Array(arrayBuffer);
		const min = Math.min(...tvs_surfaces);
		const max = Math.max(...tvs_surfaces);

		self.postMessage({ type: "tile", key, data: tvs_surfaces, min, max });
		loading.set(key, false);
	}
};

function tileKey(z, x, y) {
	return `${z}/${x}/${y}`;
}
