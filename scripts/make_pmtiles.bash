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
	merged=output/merged.tif

	echo "Using archive at $archive"

	# Merge all the heatmap GeoTiffs into one big GeoTiff.
	gdalbuildvrt "$world_vrt" "$archive"/*.tiff
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

function make_prod_pmtiles {
	local version=$1

	export WORKERS=12
	export GDAL_CACHEMAX=32768

	make_pmtiles "$version" output/world.pmtiles
}
