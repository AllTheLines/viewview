#!/usr/bin/env python3
import csv

from pyproj import CRS, Transformer
from shapely import to_geojson
from shapely.geometry import Point, Polygon, MultiPolygon
from shapely.ops import transform

def get_circle(lat, lon, rad):
    # Azimuthal equidistant projection
    aeqd_proj = CRS.from_proj4(
        f"+proj=aeqd +lat_0={lat} +lon_0={lon} +x_0=0 +y_0=0 +units=m datum=WGS84 +no_defs")
    tfmr = Transformer.from_proj(aeqd_proj, aeqd_proj.geodetic_crs)
    buf = Point(0, 0).buffer(rad)  # distance in metres
    return transform(tfmr.transform, buf)

def run():
    lower_48 = [
        (-127.91931351600928, 48.98859645722183),
        (-127.91931351600928, 25.152977183126012),
        (-66.44716959341592, 25.152977183126012),
        (-66.44716959341592, 48.98859645722183),
        (-127.91931351600928, 48.98859645722183)
    ]

    polygon = Polygon(lower_48)

    empty = MultiPolygon([])

    with open("website/public/tiles.csv") as f:
        tile_reader = csv.reader(f)
        for tile in tile_reader:
            circle = get_circle(float(tile[1]), float(tile[0]), float(tile[2]))
            if circle.intersects(polygon):
                print(f"unioning {tile[1]},{tile[0]} {tile[2]}m")
                print(f"{to_geojson(circle)}")
                # empty = empty.union(circle)
    # print(to_geojson(empty))

if __name__ == "__main__":
    run()
