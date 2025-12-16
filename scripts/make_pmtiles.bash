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
	gdal_translate "$world_vrt" "$merged"

	# Create the global `.pmtile`
	uv run scripts/to_pmtiles.py "$merged" "$output" \
		--min_zoom 0 \
		--max_zoom 11

	if [[ $version != "local" ]]; then
		rclone_put "$output" viewview/runs/"$version"/pmtiles/
	fi
}
