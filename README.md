# A View Of All Views

This repo is for all the supporting code used to find and display the longest line of sight on the planet.

The main viewshed algorithm is another repo https://github.com/tombh/total-viewsheds

The raw elevation data is, as of writing (October 2025), from https://www.viewfinderpanoramas.org/Coverage%20map%20viewfinderpanoramas_org3.htm Other sources of data are available, notably via AWS's https://registry.opendata.aws/terrain-tiles, but as far as I can tell viewfinderpanoramas.org offers a cleaned version, removing noisy data and filling in voids with other sources.

## Packer

I wrote an in-depth blog post about the Packer https://tombh.co.uk/packing-world-lines-of-sight

![Map of all the longest line of sight tiles in the world](https://alltheviews.world/world_packed.webp)

This map shows a not-terrible packing of the minimum tiles needed to guarantee searching every line of sight on the planet.

To calculate any [viewshed](https://en.wikipedia.org/wiki/Viewshed), and therefore line of sight, you must inevitably provide more data than ends up being used. This is to say that you don't know the limits of what you can see from a given point until you actually calculate it. The only limit you can calculate beforehand is the longest _theoretical_ line of sight based on the highest points within the region you're interested in.

Here are some examples, they are for 2 peaks of the same height and the maximum distance that they could see each other from:

* 8848m  670km
* 6km    553km
* 4km    452km
* 2km    319km
* 1km    226km
* 0.5km  160km
* 1.65m  9km (height of an average human)

Formula: √(2 * Earth Radius * height) * 2

So as long as you provide the raw data within these theoretical limits then you are at least guaranteed to have complete viewsheds. The worst that can happen is that RAM and CPU cycles are wasted on calculating lines that have already terminated.

We could just cover the world with hundreds of 670km x 670km squares and calculate all the lines of sight inside each one. But that's only really necessary in the Himalayas. But then if we start using different size squares (let's call them tiles), then we face the problem of them not packing well. We start to get overlaps which again introduce lots of wasted RAM and CPU cycles because we're re-calculating regions that have already been done.

So can we strike an optimal balance? This is what the `Packer` in this repo tries to do.

> [!NOTE]  
> The packer only works between ±85° latitude:
>
> We don't go all the way to ±90 because the current (October 2025) implementation doesn't
> work well when a tile crosses the North or South poles. 85°N is roughly the
> northern-most part of Greenland, beyond which there isn't really any more land. And we
> assume that beyond 85°S, which is solely in Antartica, doesn't have the longest line of
> sight on the planet.

### Steps
1. Create a "lower" resolution version of the global elevation data that for every N degrees (I chose 0.1°, or ~11km at the equator) there's a _sub_-tile that captures the highest point within itself. So it is lower resolution but it hasn't lost any critical data via the typical side effects of interpolation. These subtiles are like an accelerating data structure for all the lookups we'll be doing to find TVS tiles.
2. For each subtile on a popable stack:
    1. Create any TVS tile that fits the highest elevation of all the subtiles it covers.
    2. If the tile overlaps with other tiles discard it and move on to the next subtile.
    3. If the tile doesn't overlap then keep it and remove all the subtiles from the stack that the tile covers.
    4. Repeat this process until no more non-overlapping tiles can be found.
3. Repeat step 2 but allow for overlapping tiles.
4. Once all subtiles are covered, run some cleanup, like removing tiles that are already encompassed by larger tiles.

### Usage
1. `cargo run --release --bin tasks -- max-sub-tiles`. Creates `./max_subtiles.bin`.
2. `cargo run --release --bin tasks -- packer`. Creates a `static/tiles.json`.

## Stitcher

Creates arbitrary tiles out of the global DEM data.

```
cargo run --bin tasks -- stitch \
  --dems /publicish/dems \
  --centre -3.049208402633667,53.24937438964844 \
  --width 366610.1875
```

## Calculate Total Viewsheds

Using https://github.com/AllTheLines/CacheTVS

Outputs `.bt` heatmap.

## Prepare For Cloud

```
# Note that all these depend on a `./output` path existing.
./ctl.sh prepare_for_cloud ../total-viewsheds/output/total_surfaces.bt
./ctl.sh prepare_for_cloud ../total-viewsheds/output/longest_lines.bt

./ctl.sh make_pmtiles latest website/public/world.pmtiles
```

## Atlas

Process all the tiles for the entire planet. It manages the "Stitcher", "Total Viewsheds" and "Prepare For Cloud" steps above.

```
# Saving data locally, useful for development:
RUST_LOG=trace cargo run --bin tasks -- atlas run \
  --provider local \
  --run-id dev \
  --master website/public/tiles.csv \
  --centre -4.549624919891357,47.44954299926758 \
  --dems /publicish/dems \
  --tvs-executable ../total-viewsheds/target/release/total-viewsheds \
  --longest-lines-cogs website/public/longest_lines

# Saving data on remote machines and S3, used for production:
RUST_LOG=off,tasks=trace cargo run --bin tasks -- atlas run \
  --provider digital-ocean \
  --run-id 0.1 \
  --master website/public/tiles.csv \
  --centre -13.949958801269531,57.94995880126953 \
  --dems /publicish/dems \
  --tvs-executable /root/tvs/target/release/total-viewsheds \
  --longest-lines-cogs output/longest_lines
```

Atlas doesn't run the following commands, you'll want to manually run them after a
bunch of tiles have been processed:

```
# Create an index of all the COG (optimised GeoTiff) files that contain all the longest
# lines of sight for every point on the planet. Should only take seconds to run.
RUST_LOG=off,tasks=trace cargo run --bin tasks -- atlas longest-lines-index
```

```
# Create the gigantic (10s of GBs) global `.pmtile` that contains the TVS heatmap for
# the entire planet. This requires a machine with a lot of RAM and CPU. As of writing, with
# a ~10% world run, an 80Gb machine with 48 cores, took around 20 minutes.
#
# Replace `latest` with `local` to skip syncing files to S3.
./ctl.sh make_pmtiles latest output/world.pmtiles
```

## Website

https://alltheviews.world Still in development so expect daily breakages.

`.ctl.sh website_deploy`
