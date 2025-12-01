# /// script
# dependencies = [
#   "pmtiles==3.5.0",
#   "rasterio",
#   "rio-tiler",
#   "mercantile",
#   "numpy",
#   "tqdm",
# ]
# ///

# Inspired by:
#   https://gist.github.com/JesseCrocker/4fee23a262cdd454d14e95f2fb25137f

import argparse
import numpy
import warnings
import zlib

import mercantile
import rasterio
import itertools

from rasterio.warp import transform_bounds
from rio_tiler.io.rasterio import Reader, reader
from pmtiles.tile import Compression
from pmtiles.tile import TileType
from pmtiles.tile import zxy_to_tileid
from pmtiles.writer import Writer

from tqdm.contrib.concurrent import process_map


def slice_to_pmtiles(
    input_file,
    output_file,
    min_zoom,
    max_zoom,
):
    with rasterio.open(input_file) as src:
        src_crs = src.crs
        bbox = transform_bounds(src_crs, "EPSG:4326", *list(src.bounds))

    with open(output_file, "wb") as output_file:
        pmtiles_writer = Writer(output_file)

        tiles = mercantile.tiles(
            bbox[0], bbox[1], bbox[2], bbox[3], range(min_zoom, max_zoom + 1)
        )

        results = process_map(
            make_pmtile_data,
            tiles,
            itertools.repeat(input_file),
            unit=" tiles",
            desc="Processing tiles",
        )

        for item in results:
            tile_id, data = item
            pmtiles_writer.write_tile(tile_id, data)

        pmtiles_writer.finalize(
            header={
                "tile_compression": Compression.UNKNOWN,
                "tile_type": TileType.UNKNOWN,
                "min_lon_e7": int(bbox[0] * 1e7),
                "min_lat_e7": int(bbox[1] * 1e7),
                "max_lon_e7": int(bbox[2] * 1e7),
                "max_lat_e7": int(bbox[3] * 1e7),
            },
            metadata={},
        )


def make_pmtile_data(tile, input_file):
    options: reader.Options = {
        "resampling_method": "bilinear",
    }

    with Reader(input=input_file, options=options) as cog:
        x, y, z = tile.x, tile.y, tile.z
        try:
            tile, _ = cog.tile(x, y, z, tilesize=256)
        except Exception as error:
            print(f"WARNING: Tile {z}/{x}/{y}: ", error)
            return
        arr = tile[0]  # single band
        data = arr.astype(numpy.float32).flatten().tobytes()
        if numpy.all(data == 0.0):
            # Ignore empty tiles.
            return

        data = zlib.compress(data, level=1)
        tile_id = zxy_to_tileid(z, x, y)
        return (tile_id, data)


def ignore_warnings():
    warnings.simplefilter("ignore")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Slice raster into tiles and write to PMTiles."
    )
    parser.add_argument("input_file", help="Input raster file")
    parser.add_argument("output_file", help="Output PMTiles file")
    parser.add_argument(
        "--min_zoom", required=True, type=int, help="Minimum zoom level"
    )
    parser.add_argument(
        "--max_zoom", required=True, type=int, help="Maximum zoom level"
    )

    args = parser.parse_args()

    ignore_warnings()

    slice_to_pmtiles(
        args.input_file,
        args.output_file,
        args.min_zoom,
        args.max_zoom,
    )
