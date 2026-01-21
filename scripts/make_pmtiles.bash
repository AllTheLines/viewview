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

	echo "Using archive at $archive"

	if [ -d "$archive" ] && [ "$(ls -A "$archive")" ]; then
		echo "$archive is not empty. Exiting."
		exit 1
	fi

	prepare_all_tiffs work/raw "$archive"

	# Collate all the heatmap GeoTiffs into a single virtual file.
	gdalbuildvrt "$world_vrt" "$archive"/*.tiff

	# Create the global `.pmtile`
	uv run scripts/to_pmtiles.py "$world_vrt" "$output" \
		--min_zoom 0 \
		--max_zoom 11

	if [[ $version != "local" ]]; then
		rclone_put "$output" viewview/runs/"$version"/pmtiles/
	fi
}

# Interpolate the TVS heatmap's data to EPSG:3857.
#
# Along with all the other heatmap GeoTiff's, it will be used to create the single global
# `.pmtile`. EPSG:3857 is the Mercator metric projection, it is better for tiling.
#
# Note: We need to force the pixel size because without it then `gdal` chooses bad sizes. It
# makes tiles at the poles, for example, start reaching pixel sizes of ~500. That worst case
# is then used as the default for the _whole_ world!
#
# TODO: Use worker jobs to parallelise. Will speed it up and give retries and resumes.
function process_raw_tvs_tiff {
	set -e
	set -x

	local input=$1
	local output=$2

	# These are in an array because I've been exploring splitting tiles that cross the antimeridian.
	# Having an array makes it easier to diverge the processing into west and east branches.
	local warp_args=(
		"-overwrite"
		"-tr" "100" "100"
		"-t_srs" "EPSG:3857"
		"-dstnodata" "0"
		"-srcnodata" "0"
		"-r" "bilinear"
		"-co" "BIGTIFF=IF_SAFER"
		"-co" "COMPRESS=DEFLATE"
		"-co" "TILED=YES"
		"-co" "PREDICTOR=3"
		"$input"
	)

	warp_args=("${warp_args[@]}" "$output")
	gdalwarp "${warp_args[@]}"
}

# Create overviews to speed up tile creation at lower zoom levels.
function create_overviews_for_tiff {
	local input=$1

	gdaladdo \
		-r bilinear \
		"$input" 2 4 8 16 32 64 128 256 512 1024 2048
}

function prepare_tiff {
	set -eo pipefail
	set -x

	local source=$1
	local destination=$2

	filename=$(basename "$source")
	latitude=$(echo "$filename" | sed -E 's#.*_([0-9.-]+)\.tiff#\1#')

	if (($(echo "$latitude > -80" | bc -l))); then
		process_raw_tvs_tiff "$source" "$destination/$filename"
		create_overviews_for_tiff "$destination/$filename"
	else
		echo "Not creating preparing heatmap tiff for Antartic tile: $input"
	fi

}

function prepare_all_tiffs {
	local source_directory=$1
	local destination_directory=$2

	export -f prepare_tiff
	export -f process_raw_tvs_tiff
	export -f create_overviews_for_tiff

	mkdir -p "$destination_directory"

	find "$source_directory" -name "*.tiff" |
		parallel -j +0 --halt now,fail=1 prepare_tiff {} "$destination_directory"
}
