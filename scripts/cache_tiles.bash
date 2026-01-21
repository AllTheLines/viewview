# For zoom levels 0-8, this gets filled to nearly ~10GB
export TILE_CACHE_DIRECTORY=output/cache

# Needs `pmtiles`: https://github.com/protomaps/go-pmtiles
function cache_cacheable_tiles {
	set -e
	local version=$1

	trap 'echo "Cleaning up pmtiles serve..."; kill $(jobs -p) 2>/dev/null' EXIT
	pmtiles serve . --bucket https://cdn.alltheviews.world &
	sleep 1

	mkdir -p $TILE_CACHE_DIRECTORY
	export -f cache_tile

	# Takes about 15 minutes for zoom levels 0-8
	for z in {0..1}; do
		max=$((2 ** z - 1))
		parallel --jobs 20 cache_tile "$version $z {1} {2}" ::: $(seq 0 $max) ::: $(seq 0 $max)
	done

	# Takes just under an hour for zoom levels 0-8
	# NB: There are lots of errors for the empty 0-byte sea-based tiles. They're safe to ignore.
	rclone_put output/cache viewview/runs/"$version"/pmtiles/cache
}

function cache_tile {
	set -e

	local version=$1
	local z=$2
	local x=$3
	local y=$4
	local path="$z/$x/$y"
	local base="runs/$version/pmtiles/world.pmtiles/world"

	curl \
		--create-dirs \
		--silent \
		--show-error \
		--fail \
		--output "$TILE_CACHE_DIRECTORY/$path" \
		"http://localhost:8080/$base/$path.bin"
}
