# CRS: NAD83(CSRS) / UTM zone 10N (already metric and equally spaced pixels)
# Center: 467048.260, 5420616.975 (-123.44996829267, 48.9376084029813)
# Pixels: 87616x78341 (target for factor of 48 87648x87648)
# Resolution: 0.25m (25cm)
# Extent: 456096.26 5410824.35 478000.26 5430409.6

# How to compute:
#
# RUSTFLAGS='-Ctarget-cpu=native' cargo run --release --features=ring_data \
# 	compute dsm-1.0.tiff \
# 	--thread-count 160 \
# 	--database-per-thread \
# 	--backend cpu \
# 	--process total-surfaces,viewsheds \
# 	--observer-height 0.0 \
# 	--aoi-point -123.57209623620894,49.02110568820024 \
# 	--aoi-point -123.29132955324245,48.87887000504236 \
# 	--aoi-point -123.34211991205905,48.83884334550575 \
# 	--aoi-point -123.61305988176157,48.99841636016359 \
# 	--aoi-point -123.57209623620894,49.0211056882002 \
# 	--output-dir /mnt/disks/viewshed \
# 	--viewsheds-db-path /mnt/disks/viewshed/dbs

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

	read -r xcentre ycentre <<<"$(
		gdalinfo \
			-json "$input" | jq -r '.cornerCoordinates.center | join(" ")'
	)"

	source_crs=$(gdalsrsinfo -o wkt "$input")
	read -r lon_centre lat_centre _ <<<"$(
		echo "$xcentre" "$ycentre" | gdaltransform -s_srs "$source_crs" -t_srs EPSG:4326
	)"

	read -ra extent < <(_galiano_extent_padded)

	gdalwarp \
		-overwrite \
		-t_srs "+proj=aeqd +lat_0=$lat_centre +lon_0=$lon_centre +datum=WGS84 +units=m" \
		-te "${extent[0]}" "${extent[1]}" "${extent[2]}" "${extent[3]}" \
		-tr "$resolution" "$resolution" \
		-r bilinear \
		-dstnodata 0 \
		-co COMPRESS=ZSTD \
		-co ZSTD_LEVEL=9 \
		"$input" \
		"$output"

	pad_to_factor "$output"

}

function _galiano_extent_padded {
	local original_scale=0.25
	local auxiliary=3
	local square_to_circle=1.414 # Make the circle _outside_ the DEM
	width=$(gdalinfo -json "$input" | jq '.size[0]')
	width_all=$(echo "$width * $square_to_circle * $auxiliary" | bc)
	extend_meters=$(
		echo "($width_all * $original_scale) / 2" | bc
	)

	xmin=-$extend_meters
	ymin=-$extend_meters
	xmax=$extend_meters
	ymax=$extend_meters

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
