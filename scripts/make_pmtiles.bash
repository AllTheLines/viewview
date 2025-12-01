# Create the single global `.pmtile` for the whole world
function make_pmtiles {
	set -e
	set -x

	# The version of the world run we're on.
	local version=$1

	# Output `.pmtile` tile.
	local output=$2

	ensure_tiles_env

	rm "$output" || true
	rm -f "$TMP_DIR/"*
	merged=$TMP_DIR/merged.tif

	echo "Using archive at $ARCHIVE_DIR"

	# Merge all the heatmap GeoTiffs into one big GeoTiff.
	gdal_merge \
		-n 0 \
		-a_nodata 0 \
		-co ALPHA=YES \
		-o "$merged" \
		"$ARCHIVE_DIR"/*.tiff

	# Create the global `.pmtile`
	uv run scripts/to_pmtiles.py "$merged" "$output" \
		--min_zoom 0 \
		--max_zoom 11

	if [[ $version == "latest" ]]; then
		version=$(get_current_run_id)
	fi

	if [[ $version != "local" ]]; then
		s3 put "$output" s3://viewview/runs/"$version"/pmtiles/world.pmtiles
	fi
}
