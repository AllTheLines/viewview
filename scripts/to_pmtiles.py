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
import os
from concurrent.futures import ProcessPoolExecutor

import mercantile
import rasterio

from rasterio.warp import transform_bounds
from rio_tiler.io.rasterio import Reader
from pmtiles.tile import Compression
from pmtiles.tile import TileType
from pmtiles.tile import zxy_to_tileid
from pmtiles.writer import Writer


_global_cog = None


def init_worker(input_file):
    global _global_cog
    _global_cog = Reader(input=input_file, options={"resampling_method": "bilinear"})


def slice_to_pmtiles(
    input_file,
    output_file,
    min_zoom,
    max_zoom,
):
    with rasterio.open(input_file) as src:
        src_crs = src.crs
        bbox = transform_bounds(
            src_crs, "EPSG:4326", *list(src.bounds), densify_pts=300
        )

    with open(output_file, "wb") as output_file:
        pmtiles_writer = Writer(output_file)

        tiles = mercantile.tiles(
            bbox[0], bbox[1], bbox[2], bbox[3], range(min_zoom, max_zoom + 1)
        )

        workers = (os.cpu_count() or 2) // 2
        with ProcessPoolExecutor(
            max_workers=workers, initializer=init_worker, initargs=(input_file,)
        ) as pool:
            for result in pool.map(make_pmtile_data, tiles, chunksize=50):
                if result is None:
                    continue
                tile_id, data = result
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


def make_pmtile_data(tile):
    if _global_cog is None:
        print("Merged global COG file not opened yet.")
        return

    x, y, z = tile.x, tile.y, tile.z
    try:
        tile, _ = _global_cog.tile(x, y, z, tilesize=256)
    except Exception as error:
        print(f"WARNING: Tile {z}/{x}/{y}: ", error)
        return

    data = tile[0].ravel().tobytes(order="C")
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
