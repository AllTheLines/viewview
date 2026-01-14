//! Create an overview of the world's longest lines.
//!
//! We grid the world using Uber's hexagonal H3 grids. We use zoom level 4 which means there are a
//! total of 341,162 grids, each one covering around 65kmx65km. For each grid we find the longest
//! line of sight in it. We then save all the lines of sight, just their coordinates and length,
//! in a single bin file for use by the website.

use std::sync::Arc;

use color_eyre::{Result, eyre::ContextCompat as _};

/// A longest in a single H3 grid.
#[derive(Debug, Clone, Copy)]
struct LongestLine {
    /// The coordinates of the line.
    lonlat: crate::projector::LonLatCoord,
    /// The raster coordinates of the line in its containing COG. Used mostly for debugging
    /// because different COG can contain the same line.
    coord: geo::Coord,
    /// The bit-packed data for the line (angle and distance).
    packed: super::packed::LineOfSight,
}

/// A hash where each key is an H3 grid cell and its value is the longest line of sight within
/// that grid cell.
type HashMap = std::collections::HashMap<h3o::CellIndex, LongestLine>;

/// Keep track of the current longest line in each H3 grid.
type StateHash = Arc<tokio::sync::RwLock<HashMap>>;

/// Representation of a longest lines COG file.
struct Tile {
    /// The AEQD to lon/lat coordinate converter
    projector: crate::projector::Convert,
    /// The raw point data for the tile
    buffer: gdal::raster::Buffer<f32>,
    /// The width of the tile in points
    width: usize,
    /// Offset to the centre of the tile in points.
    offset: f64,
}

impl Tile {
    /// Entrypoint.
    fn process(path: &std::path::Path) -> Result<HashMap> {
        let centre = Self::parse_centre_coords(path)?;
        let buffer = Self::load(path)?;
        let width = buffer.width();
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "There's no other way"
        )]
        let offset = width as f64 / 2.0f64;
        let tile = Self {
            projector: crate::projector::Convert { base: centre },
            buffer,
            width,
            offset,
        };

        let local = tile.find_longest_lines()?;

        Ok(local)
    }

    /// Get the lon/lat coordinates representing the centre of the longest lines COG.
    fn parse_centre_coords(path: &std::path::Path) -> Result<crate::projector::LonLatCoord> {
        let filestem = path
            .file_stem()
            .context("Couldn't get file stem of longest lines tile")?
            .display()
            .to_string();
        let parts: Vec<&str> = filestem.split('_').collect();

        #[expect(
            clippy::get_first,
            reason = "Better to have the consistent access method."
        )]
        let lon: f64 = parts
            .get(0)
            .context("Longitude not present in path parts")?
            .parse()?;
        let lat: f64 = parts
            .get(1)
            .context("Latitude not present in path parts")?
            .parse()?;
        let coord = crate::projector::LonLatCoord(geo::coord! {x: lon, y: lat});

        Ok(coord)
    }

    /// Load a longest lines COG file.
    fn load(path: &std::path::Path) -> Result<gdal::raster::Buffer<f32>> {
        tracing::trace!("Loading {path:?}");

        let dataset = gdal::Dataset::open(path)?;
        let band = dataset.rasterband(1)?;

        let buffer: gdal::raster::Buffer<f32> =
            band.read_as((0, 0), band.size(), band.size(), None)?;

        Ok(buffer)
    }

    /// Iterate through every longest line in COG, associate it with a H3 grid, check to see if
    /// it's the longest in the grid and set record it if so.
    fn find_longest_lines(&self) -> Result<HashMap> {
        tracing::trace!("Looping through {} points", self.buffer.len());

        let mut local = HashMap::new();
        for (index, &value) in self.buffer.data().iter().enumerate() {
            let coord = self.index_to_coord(index);
            let lonlat = self.coord_to_lonlat(coord)?;
            let cell = h3o::LatLng::new(lonlat.0.y, lonlat.0.x)?.to_cell(h3o::Resolution::Four);

            let current = super::packed::LineOfSight(value);
            let longest_line = LongestLine {
                coord,
                lonlat,
                packed: current,
            };

            let Some(existing) = local.get(&cell) else {
                local.insert(cell, longest_line);
                continue;
            };

            if current.distance() > existing.packed.distance() {
                local.insert(cell, longest_line);
            }
        }

        Ok(local)
    }

    /// Convert a 1D index to a 2D coordinate.
    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "There's no other way"
    )]
    const fn index_to_coord(&self, index: usize) -> geo::Coord {
        geo::coord! {
            x: index.rem_euclid(self.width) as f64,
            y: index.div_euclid(self.width) as f64,
        }
    }

    /// Convert the index of a single point in a tile to its lon/lat coordinate.
    fn coord_to_lonlat(&self, point_coord: geo::Coord) -> Result<crate::projector::LonLatCoord> {
        #[expect(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            reason = "There's no other way"
        )]
        let flipper = self.width as f64 - 1.0f64;
        let aeqd_coord = geo::coord! {
            x: (point_coord.x - self.offset) * 100.0f64,
            y: (flipper - point_coord.y - self.offset) * 100.0f64,
        };
        self.projector.to_degrees(aeqd_coord)
    }
}

pub const LONGEST_LINES_GRIDED_FILENAME: &str = "longest_lines_grided.bin";

/// Sync the longest lines grid to S3.
async fn sync_to_s3(source: std::path::PathBuf, run_id: &str) -> Result<()> {
    let destination =
        format!("s3://viewview/runs/{run_id}/longest_lines_cogs/{LONGEST_LINES_GRIDED_FILENAME}");
    crate::atlas::machines::local::Machine::connection()
        .sync_file_to_s3(&source.display().to_string(), &destination)
        .await?;

    Ok(())
}

/// Entrypoint.
pub async fn run(config: &crate::config::LongestLinesOverviews) -> Result<()> {
    let workers = crate::config::number_of_cpus_on_machine() - 1;
    let world: StateHash = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    let tiffs = find_longest_lines_tiffs_in_path(&config.tiffs)?;
    let jobs = Arc::new(tokio::sync::RwLock::new(tiffs));

    let mut handles = Vec::new();
    for worker in 0..workers {
        let world_clone = Arc::clone(&world);
        let jobs_clone = Arc::clone(&jobs);
        let handle = tokio::task::spawn(async move {
            tracing::debug!("Spawning worker {worker}");

            loop {
                let job = jobs_clone.write().await.pop();
                match job {
                    Some(tiff) => {
                        let result: Result<()> = async {
                            let local = Tile::process(&tiff)?;
                            update_state(&world_clone, local).await?;
                            Ok(())
                        }
                        .await;
                        if let Err(error) = result {
                            tracing::error!("Error running longest lines job: {error}");
                            #[expect(clippy::exit, reason = "Everything needs to work!")]
                            std::process::exit(1);
                        }
                    }
                    None => {
                        tracing::debug!("Worker {worker} shutting down");
                        break;
                    }
                }
            }
        });

        handles.push(handle);
    }

    for result in futures::future::join_all(handles).await {
        result?;
    }

    sync_to_s3(
        format!(
            "./output/{}",
            LONGEST_LINES_GRIDED_FILENAME
        )
        .into(),
        &config.run_id,
    )
    .await?;

    Ok(())
}

/// Take all the newly found longest lines in a COG and check to see if they're longer than any of
/// the currently recorded global ones.
async fn update_state(world: &StateHash, local: HashMap) -> Result<()> {
    #[expect(clippy::iter_over_hash_type, reason = "We don't mind about ordering")]
    for (cell, line) in local {
        if let Some(existing) = world.write().await.get_mut(&cell) {
            if line.packed.distance() > existing.packed.distance() {
                *existing = line;
            }
        } else {
            world.write().await.insert(cell, line);
        }
    }

    let mut longest = LongestLine {
        lonlat: crate::projector::LonLatCoord(geo::Coord::zero()),
        coord: geo::Coord::zero(),
        packed: crate::atlas::longest_lines::packed::LineOfSight(0.0),
    };

    let mut grided = Vec::new();
    let count = world.read().await.len();
    #[expect(clippy::iter_over_hash_type, reason = "We don't mind about ordering")]
    for entry in world.read().await.values() {
        if entry.packed.distance() > longest.packed.distance() {
            longest = *entry;
        }

        #[expect(
            clippy::cast_possible_truncation,
            clippy::as_conversions,
            reason = "f32 is enough fidelity. And as long as the cast is consistent, we're good."
        )]
        grided.push(super::grided::Grided {
            lon: entry.lonlat.0.x as f32,
            lat: entry.lonlat.0.y as f32,
            distance: entry.packed.distance(),
        });
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        reason = "Just for debugging"
    )]
    {
        tracing::info!(
            "Current longest (out of {count}): {:?} {}km {}° https://alltheviews.world/longest/{},{}",
            longest.coord,
            longest.packed.distance() as f32 / 1000.0,
            longest.packed.angle()?,
            longest.lonlat.0.x,
            longest.lonlat.0.y
        );
    };

    super::grided::write(
        format!(
            "./output/{}",
            LONGEST_LINES_GRIDED_FILENAME
        )
        .into(),
        &grided,
    )
    .await?;

    Ok(())
}

/// Find all the `.tiff`s in a given directory.
fn find_longest_lines_tiffs_in_path(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();

    for fallable_entry in std::fs::read_dir(dir)? {
        let entry = fallable_entry?;
        let path = entry.path();

        if path.is_file() {
            let extension = path.extension().and_then(|extension| extension.to_str());
            if extension == Some("tiff") {
                files.push(path);
            }
        }
    }

    Ok(files)
}
