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

from multiprocessing import Pool, Queue
from queue import Empty
import threading
from threading import Event

import mercantile
import rasterio

from rasterio.warp import transform_bounds
from rio_tiler.io.rasterio import Reader
from pmtiles.tile import Compression
from pmtiles.tile import TileType
from pmtiles.tile import zxy_to_tileid
from pmtiles.writer import Writer
import tqdm

# I think this helps because we're already controlling concurrency by other means.
os.environ["GDAL_NUM_THREADS"] = "1"

# The master merged GeoTiff of the whole world. Can be 100s of GBs
_global_cog = None
# The worker queue.
_output_queue = None


def init_worker(input_file, queue):
    global _global_cog, _output_queue
    _output_queue = queue
    _global_cog = Reader(input=input_file, options={"resampling_method": "bilinear"})


def slice_to_pmtiles(input_file, output_file, min_zoom, max_zoom):
    with rasterio.open(input_file) as src:
        bbox = transform_bounds(src.crs, "EPSG:4326", *src.bounds, densify_pts=300)

    tile_iter = mercantile.tiles(
        bbox[0],
        bbox[1],
        bbox[2],
        bbox[3],
        range(min_zoom, max_zoom + 1),
    )

    # Maximum number of tiles in RAM at a time.
    queue = Queue(maxsize=128)

    with open(output_file, "wb") as f:
        writer = Writer(f)

        stop_event = Event()
        thread = threading.Thread(
            target=writer_thread,
            args=(queue, writer, stop_event),
            daemon=True,
        )
        thread.start()

        workers = min(8, os.cpu_count() or 2 // 2)
        if os.environ.get("WORKERS"):
            workers = int(os.environ["WORKERS"])

        with Pool(
            processes=workers,
            initializer=init_worker,
            initargs=(input_file, queue),
        ) as pool:
            for _ in tqdm.tqdm(
                pool.imap_unordered(make_pmtile_data, tile_iter, chunksize=1),
                total=count_tiles(bbox, min_zoom, max_zoom),
                desc="Generating tiles",
            ):
                # work happens via queue
                pass

        stop_event.set()
        queue.put(None)
        thread.join()

        writer.finalize(
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

    band = tile[0]
    if not band.any():
        return

    data = numpy.ascontiguousarray(band).tobytes()
    compressed = zlib.compress(data, level=1)
    tile_id = zxy_to_tileid(z, x, y)

    if _output_queue is None:
        print("Worker queue hasn't been instantiated yet.")
        return

    _output_queue.put((tile_id, compressed))


def writer_thread(queue, writer, stop_event):
    while not stop_event.is_set():
        try:
            item = queue.get(timeout=0.1)
        except Empty:
            continue

        if item is None:
            break

        tile_id, data = item
        writer.write_tile(tile_id, data)


def ignore_warnings():
    warnings.simplefilter("ignore")


# Just to help with showing progress
def count_tiles(bbox, min_zoom, max_zoom):
    return sum(
        1
        for _ in mercantile.tiles(
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            range(min_zoom, max_zoom + 1),
        )
    )


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
