function make_pmtiles {
	set -e
	set -x

	local input=$1
	local output=$2

	longitude=$(gdalinfo -json "$input" | jq '.geoTransform[0]')
	latitude=$(gdalinfo -json "$input" | jq '.geoTransform[3]')
	pixel_width=$(gdalinfo -json "$input" | jq '.size[0]')
	width=$((pixel_width * 50))

	OUTPUT_DIR=output
	ARCHIVE_DIR=$OUTPUT_DIR/archive
	TMP_DIR=$OUTPUT_DIR/tmp

	merged=$TMP_DIR/merged.tif
	plain_tif=$TMP_DIR/plain.tif
	cog=$TMP_DIR/cog.tif
	archive=$ARCHIVE_DIR/"$longitude"_"$latitude".tiff

	mkdir -p $ARCHIVE_DIR
	mkdir -p $TMP_DIR
	rm -f $TMP_DIR/*
	rm -f "$output"

	gdal_translate -of GTiff "$input" $plain_tif
	gdal_edit \
		-a_ullr "-$width" "$width" "$width" "-$width" \
		-a_srs "+proj=aeqd +lat_0=$latitude +lon_0=$longitude +datum=WGS84" \
		$plain_tif

	gdalwarp \
		-overwrite \
		-t_srs EPSG:3857 \
		-r bilinear \
		-co COMPRESS=DEFLATE \
		-co ZLEVEL=9 \
		-co TILED=YES \
		-co PREDICTOR=3 \
		$plain_tif "$archive"

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
		$cog

	uv run scripts/cog_to_pmtiles.py $cog "$output" \
		--min_zoom 0 \
		--max_zoom 11
}
