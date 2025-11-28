function prepare_for_cloud {
	set -e
	set -x

	# Input `.bt` tile
	local input=$1
	# Output `.tiff` tile
	local output=$2

	ensure_tiles_env

	# Lon/lat of the input tile
	longitude=$(gdalinfo -json "$input" | jq '.geoTransform[0]')
	latitude=$(gdalinfo -json "$input" | jq '.geoTransform[3]')
	# Pixel resolution of the input tile
	pixel_width=$(gdalinfo -json "$input" | jq '.size[0]')
	# Width of the input tile
	width=$((pixel_width * 50))

	if [ -z "${output:-}" ]; then
		output="$longitude"_"$latitude".tiff
		echo "⚠️ Using tile's internal lon/lat for filename."
		echo "⚠️ This may not match the coordinates of the Packer-defined tile."
	fi

	# Where to keep the longest lines COG files.
	LONGEST_LINES_DIR=${LONGEST_LINES_DIR:-$OUTPUT_DIR/longest_lines}

	# Just the filename without its path
	base=$(basename "$input")
	# The filename without its extension
	stem="${base%.*}"

	mkdir -p "$LONGEST_LINES_DIR"
	mkdir -p "$TMP_DIR"
	rm -f "$TMP_DIR/"*

	plain_tif=$TMP_DIR/plain.tif
	archive=$ARCHIVE_DIR/"$output"

	gdal_translate -of GTiff -a_nodata 0 "$input" "$plain_tif"
	gdal_edit \
		-a_ullr "-$width" "$width" "$width" "-$width" \
		-a_srs "+proj=aeqd +lat_0=$latitude +lon_0=$longitude +datum=WGS84" \
		"$plain_tif"

	if [[ $stem == "longest_lines" ]]; then
		cog=$LONGEST_LINES_DIR/"$output"
		gdal_translate \
			-of COG \
			-co BLOCKSIZE=32 \
			-co RESAMPLING=NEAREST \
			-co OVERVIEWS=NONE \
			-co COMPRESS=DEFLATE \
			-co PREDICTOR=3 \
			"$plain_tif" "$cog"
	fi

	if [[ $stem == "total_surfaces" ]]; then
		gdalwarp \
			-overwrite \
			-t_srs EPSG:3857 \
			-dstnodata 0 \
			-srcnodata 0 \
			-r nearest \
			-co COMPRESS=DEFLATE \
			-co TILED=YES \
			-co PREDICTOR=3 \
			"$plain_tif" "$archive"

	fi
}

function get_current_run_id {
	jq --raw-output '.run_id' "$PROJECT_ROOT"/output/status.json
}
