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
    return transform(tfmr.transform, buf).exterior

def run():
    lower_48 = [
        [
            -125.01199211318004,
            49.41218030132055
          ],
          [
            -124.4419707532902,
            31.981302278998214
          ],
          [
            -115.52041082447283,
            32.179847968568126
          ],
          [
            -108.85953882044555,
            30.78852357918889
          ],
          [
            -97.22939319780437,
            25.497087631699486
          ],
          [
            -80.23684222992347,
            24.853646547454247
          ],
          [
            -79.42700235393075,
            28.102743189597817
          ],
          [
            -66.4244207786077,
            44.873920605365896
          ],
          [
            -68.0376666519928,
            47.59634997835687
          ],
          [
            -69.16061164615257,
            47.498091892542845
          ],
          [
            -82.62066406484261,
            41.82988843150804
          ],
          [
            -81.37936206832772,
            46.07305985341941
          ],
          [
            -91.00969176340848,
            49.1827838565265
          ],
          [
            -95.02488850957758,
            49.43229447990208
          ],
          [
            -95.88867664299451,
            49.07804672641876
          ],
          [
            -125.01456739757708,
            49.40661002406026
          ]
    ]

    polygon = Polygon(lower_48)

    empty = MultiPolygon([])

    with open("website/public/tiles.csv") as f:
        tile_reader = csv.reader(f)
        for tile in tile_reader:
            circle = get_circle(float(tile[1]), float(tile[0]), float(tile[2]) / 2.)
            coords = f"{tile[0]}_{tile[1]}"

            if (not circle.intersects(polygon)) or (coords == "179.3502960205078_51.349456787109375"):
                continue

            print(f"{tile[0][:8]}*_{tile[1][:8]}*.tiff")
            # empty = empty.union(circle)
        # print(to_geojson(empty))

if __name__ == "__main__":
    run()
