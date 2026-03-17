# CRS: NAD83(CSRS) / UTM zone 10N (already metric and equally spaced pixels)
# Center: 467048.260, 5420616.975 (123d26'59.83"W, 48d56'15.37"N)
# Pixels: 87616x78341 (target for factor of 48 87648x87648)
# Resolution: 0.25m (25cm)
# Extent: 456096.26 5410824.35 478000.26 5430409.6

# Convert the Galiano DSM to:
#   * a square
#   * a sauare with a width of factor 48
#   * the given resolution
#
# Usage: `./ctl.sh galiano_prepare scratch/Galiano/dsm.tif scratch/Galiano/dsm-1.0.tiff 1.0`
function galiano_prepare {
	local input=$1
	local output=$2
	local resolution=$3

	read -ra extent < <(_galiano_extent_padded)

	gdalwarp \
		-overwrite \
		-te "${extent[0]}" "${extent[1]}" "${extent[2]}" "${extent[3]}" \
		-tr "$resolution" "$resolution" \
		-r bilinear \
		-dstnodata 0 \
		-co COMPRESS=ZSTD \
		-co ZSTD_LEVEL=9 \
		"$input" \
		"$output"

}

function _galiano_extent_padded {
	width=$(gdalinfo -json "$input" | jq '.size[0]')
	width_outer=$(echo "$width * 1.414" | bc) # Make the circle _outside_ the DEM
	target_pixels=$(next_factor "$width_outer")
	extend_meters=$(echo "$target_pixels * 0.25 * 1.5" | bc)
	read -r xcentre ycentre <<<"$(
		gdalinfo \
			-json "$input" | jq -r '.cornerCoordinates.center | join(" ")'
	)"
	xmin=$(echo "$xcentre - $extend_meters" | bc)
	ymin=$(echo "$ycentre - $extend_meters" | bc)
	xmax=$(echo "$xcentre + $extend_meters" | bc)
	ymax=$(echo "$ycentre + $extend_meters" | bc)

	echo "$xmin $ymin $xmax $ymax"
}

function next_factor {
	local width=$1
	local factor=48
	echo "((($width + $factor - 1) / $factor) * $factor)" | bc
}

function pad_to_factor {
	local input=$1
	temp=$(dirname "$input")/tmp.tiff

	width=$(gdalinfo -json "$input" | jq '.size[0]')
	next_by_factor=$(next_factor "$width")
	gdal_translate \
		-srcwin 0 0 "$next_by_factor" "$next_by_factor" \
		-a_nodata 0 \
		-co COMPRESS=ZSTD \
		-co ZSTD_LEVEL=9 \
		"$input" "$temp"

	mv "$temp" "$input"
}
