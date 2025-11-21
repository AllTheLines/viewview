function make_pmtiles {
	set -e
	set -x

	# Input `.bt` tile
	local input=$1
	# Output `.pmtile` tile.
	local output=$2

	# Lon/lat of the input tile
	longitude=$(gdalinfo -json "$input" | jq '.geoTransform[0]')
	latitude=$(gdalinfo -json "$input" | jq '.geoTransform[3]')
	# Pixel resolution of the input tile
	pixel_width=$(gdalinfo -json "$input" | jq '.size[0]')
	# Width of the input tile
	width=$((pixel_width * 50))

	# Where to save output
	OUTPUT_DIR=output
	# Where to keep previous COGs so that aggregated `.pmtile`s can be rebuilt.
	ARCHIVE_DIR=$OUTPUT_DIR/archive
	# Where to keep the longest lines COG files.
	LONGEST_LINES_DIR=$OUTPUT_DIR/longest_lines
	# Temp space
	TMP_DIR=$OUTPUT_DIR/tmp

	base=$(basename "$input")
	stem="${base%.*}"

	if [[ $stem == "total_surfaces" ]]; then
		cog=$TMP_DIR/cog.tif
	elif [[ $stem == "longest_lines" ]]; then
		cog=$LONGEST_LINES_DIR/"$longitude"_"$latitude".tiff
	else
		echo "Neither a total surfaces, nor longest lines, file."
		exit
	fi

	plain_tif=$TMP_DIR/plain.tif
	merged=$TMP_DIR/merged.tif
	archive=$ARCHIVE_DIR/"$longitude"_"$latitude".tiff

	mkdir -p $ARCHIVE_DIR
	mkdir -p $LONGEST_LINES_DIR
	mkdir -p $TMP_DIR
	rm -f $TMP_DIR/*
	rm -f "$output"

	gdal_translate -of GTiff "$input" "$plain_tif"
	gdal_edit \
		-a_ullr "-$width" "$width" "$width" "-$width" \
		-a_srs "+proj=aeqd +lat_0=$latitude +lon_0=$longitude +datum=WGS84" \
		"$plain_tif"

	if [[ $stem == "longest_lines" ]]; then
		gdal_translate \
			-of COG \
			-co BLOCKSIZE=32 \
			-co RESAMPLING=MODE \
			-co OVERVIEWS=NONE \
			-co COMPRESS=DEFLATE \
			"$plain_tif" "$cog"

		exit 0
	fi

	gdalwarp \
		-overwrite \
		-t_srs EPSG:3857 \
		-r bilinear \
		-co COMPRESS=DEFLATE \
		-co TILED=YES \
		-co PREDICTOR=3 \
		"$plain_tif" "$archive"

	gdal_merge \
		-n 0 \
		-a_nodata 0 \
		-co ALPHA=YES \
		-o $merged \
		$ARCHIVE_DIR/*.tiff

	gdal_translate \
		-of COG \
		-co PREDICTOR=3 \
		-co RESAMPLING=AVERAGE \
		"$merged" \
		"$cog"

	uv run scripts/cog_to_pmtiles.py "$cog" "$output" \
		--min_zoom 0 \
		--max_zoom 11
}
