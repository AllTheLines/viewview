# /// script
# dependencies = [
#   "rasterio",
#   "numpy",
# ]
# ///

# Helper script to make small sample GeoTiffs for testing etc.

import numpy as np
import rasterio
from rasterio.transform import from_origin

# fmt: off
x = 15
dem = [
    0,0,0,0, 0,0,0,0, 0,0,0,1,
    0,1,1,1, 1,1,1,1, 0,1,0,0,
    0,1,3,3, 2,2,2,3, 4,3,0,0,
    0,1,3,4, 3,3,3,3, 3,2,1,0,

    0,1,3,4, 5,5,5,4, 3,2,1,0,
    0,1,3,4, 4,0,6,5, 4,2,1,0,
    0,1,1,2, 5,9,x,7, 5,2,1,0,
    0,1,1,4, 4,4,4,6, 3,2,1,0,

    0,0,3,4, 4,4,5,4, 0,0,0,0,
    0,3,3,4, 3,2,3,3, 0,0,0,0,
    0,0,3,4, 2,2,1,2, 0,0,0,0,
    3,0,2,3, 1,0,1,0, 0,0,0,0,
]
# fmt: on

data = np.array(dem, dtype="uint16").reshape(12, 12)

transform = from_origin(0, 12, 1, 1)

with rasterio.open(
    "sample_16x16.tiff",
    "w",
    driver="GTiff",
    height=12,
    width=12,
    count=1,
    dtype=data.dtype,
    crs="EPSG:4326",
    transform=transform,
) as dst:
    dst.write(data, 1)

print("Created sample_16x16.tiff successfully.")

# gdal_edit.py \
#   -a_srs "+proj=aeqd +lat_0=51.4898 +lon_0=-3.123 +units=m" \
#   -a_ullr -6 6 6 -6 \
#   sample_16x16.tiff
