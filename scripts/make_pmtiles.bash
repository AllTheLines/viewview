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

	# Collate all the heatmap GeoTiffs into a single virtual file.
	gdalbuildvrt "$world_vrt" "$archive"/*.tiff

	# Create overviews to speed up tile creation at lower zoom levels.
	# Notes:
	#  * Compression would be good, but I've seen it cause segfaults.
	#  * I tried to `parallel` this, but not only is not faster but the final overviews aren't
	#    usable by our pmtiler.
	gdaladdo \
		-r bilinear \
		"$world_vrt" 2 4 8 16 32 64 128 256

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
function reproject_raw_tvs_tiff {
	local input=$1
	local output=$2

	# EPSG:3857 but allowing wrapping over ±180°
	wrapping_web_mercator="\
	  +proj=merc +a=6378137 +b=6378137 +lat_ts=0.0 +lon_0=0 \
    +over +x_0=0.0 +y_0=0 +k=1.0 +units=m +nadgrids=@null \
    +wktext +lon_wrap=-180 +no_defs\
	"
	latitude=$(echo "$input" | sed -E 's#.*_([0-9.-]+)\.tiff#\1#')

	if (($(echo "$latitude > -80" | bc -l))); then
		gdalwarp \
			-overwrite \
			-tr 100 100 \
			-t_srs "$wrapping_web_mercator" \
			-dstnodata 0 \
			-srcnodata 0 \
			-r bilinear \
			-co BIGTIFF=IF_SAFER \
			-co COMPRESS=DEFLATE \
			-co TILED=YES \
			-co PREDICTOR=3 \
			"$input" "$output"
	else
		echo "Not creating TVS heatmap tiff for Antartic tile: $input"
	fi
}

function reproject_raw_tvs_tiffs {
	local source=$1
	local destination=$2

	mkdir -p "$destination"

	for path in "$source"/*.tiff; do
		[[ -f $path ]] || continue
		file=$(basename "$path")
		reproject_raw_tvs_tiff "$path" "$destination/$file"
	done
}
