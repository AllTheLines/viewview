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

data = np.arange(100).reshape((10, 10)).astype("uint16")

transform = from_origin(0, 10, 1, 1)

with rasterio.open(
    "sample_10x10.tif",
    "w",
    driver="GTiff",
    height=10,
    width=10,
    count=1,
    dtype=data.dtype,
    crs="EPSG:4326",
    transform=transform,
) as dst:
    dst.write(data, 1)

print("Created sample_10x10.tif successfully.")
