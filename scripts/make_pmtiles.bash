# Create the single global `.pmtile` for the whole world
function make_pmtiles {
	set -e
	set -x

	# The version of the world run we're on.
	local version=$1

	# Output `.pmtile` tile.
	local output=$2

	ensure_tiles_env

	if [[ $version == "latest" ]]; then
		version=$(get_current_run_id)
	fi

	rm "$output" || true
	archive="$ARCHIVE_DIR"
	world_vrt="$archive"/world.vrt
	merged="$OUTPUT_DIR"/merged.tif

	echo "Using archive at $archive"

	# Collate all the heatmap GeoTiffs into a single virtual file.
	gdalbuildvrt "$world_vrt" "$archive"/*.tiff

	# Merge all the heatmap GeoTiffs into one big GeoTiff.
	#
	# Note: Even though this is just a temporary file, compression is still useful to help reduce
	# disk IO. Bear in mind that `merged.tif` is constantly queried by the final pmtiler, so any
	# reduction in disk IO can speed up the total runtime.
	#
	# TODO: Most of the time in this command is spent single threaded, which means up to 5 hours
	# for the whole world. Perhaps produce multiple `merged-part.tiff` as pre-step, then merge
	# for the whole world. Perhaps produce multiple `merged-part.tiff` as pre-step, then merge
	# those for the final step?
	gdal_translate \
		-co TILED=YES \
		-co BIGTIFF=YES \
		-co BLOCKXSIZE=512 \
		-co BLOCKYSIZE=512 \
		-co COMPRESS=ZSTD \
		--config GDAL_NUM_THREADS ALL_CPUS \
		--config NUM_THREADS ALL_CPUS \
		"$world_vrt" "$merged"

	# Create overviews to speed up tile creation at lower zoom levels.
	gdaladdo \
		-r bilinear \
		--config BIGTIFF YES \
		--config COMPRESS_OVERVIEW DEFLATE \
		--config GDAL_NUM_THREADS ALL_CPUS \
		"$merged" \
		2 4 8 16 32 64 128 256

	# Create the global `.pmtile`
	uv run scripts/to_pmtiles.py "$merged" "$output" \
		--min_zoom 0 \
		--max_zoom 11

	if [[ $version != "local" ]]; then
		rclone_put "$output" viewview/runs/"$version"/pmtiles/
	fi
}
