
# FAQs

## What Assumptions Did You Make?

* The original source of our data is from [NASA's SRTM survey](https://www.earthdata.nasa.gov/data/instruments/srtm) which is ~100m resolution analysis of the planet's elevation data. It is known to have some issues so we used a clean version kindly provided by [viewfinderpanoramas.org](https://www.viewfinderpanoramas.org/Coverage%20map%20viewfinderpanoramas_org3.htm).
* We used a globe earth meaning it is an approximation of earth's shape, as it is an oblate spheroid.
* We take refraction into account, and we use what the GIS community has calculated to be the world average, which is a refraction coefficient of `0.13`.
* Each viewshed is calculated using 360 lines of sight each seperated by 1°. This could potentially miss some longest lines of sight, but it is considered to be the optimal resolution to balance the accumulation of errors and computational costs. For more details, see: Siham Tabik, Antonio R. Cervilla, Emilio Zapata, Luis F. Romero in their 2014 paper _Efficient Data Structure and Highly Scalable Algorithm for Total-Viewshed Computation_ https://ieeexplore.ieee.org/document/6837455
* All computation is done on [AEQD](https://en.wikipedia.org/wiki/Azimuthal_equidistant_projection) reprojections of the raw data. For the longest lines of sight on the planet, ~500km, the worst case errors caused by this projection can reach ~0.0685%. This error is only relevant to viewsheds at the edge of the computable area of the tile, therefore those viewsheds around 500km from the centre of the tile.



## Is The Source Code Available?

Yes. [The core algorithm](https://github.com/AllTheLines/CacheTVS). [The pipeline and web app](https://github.com/AllTheLines/viewview).

