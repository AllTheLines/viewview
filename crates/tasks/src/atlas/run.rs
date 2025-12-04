//! The tool to manage all the tasks involved in crunching the whole world.
//!
//! 1. Load tiles.csv
//! 2. Get tiles around a point that haven't been crunched yet
//!   1. Stitch the relevant tile
//!   2. TVS the tile
//! 3. Run `.ctl.sh prepare_for_cloud` for `total_surfaces.bt` and `longest_lines.bt`.
//!
//! These can and probably should be run manually after a bunch of tiles have been processed.
//! 4. Create the new `world.pmtile`: `./ctl.sh make_pmtiles latest website/public/world.pmtiles`

use color_eyre::{Result, eyre::ContextCompat as _};

use apalis::prelude::TaskSink as _;

/// All the config and state needed for processing all the tiles in the world.
pub struct Atlas {
    /// The list of master tiles created by the Packer. Geo-indexed so we can iterate as a distance
    /// from a coordinate.
    tiles: rstar::RTree<crate::packer::TileRstar>,
}

impl Atlas {
    /// Instantiate.
    pub fn new(config: &crate::config::Atlas) -> Result<Self> {
        let mut tiles = rstar::RTree::new();
        let master_tiles = Self::load_master_tiles(&config.master)?;
        for master_tile in master_tiles {
            tiles.insert(crate::packer::TileRstar::new(
                master_tile.centre,
                master_tile,
            ));
        }

        let atlas = Self { tiles };
        Ok(atlas)
    }

    /// Load the master tile file created by `Packer`. This is what we iterate through to get all
    /// the tiles for the whole world.
    fn load_master_tiles(master_path: &std::path::PathBuf) -> Result<Vec<crate::tile::Tile>> {
        let mut tiles = Vec::new();
        let master_file = std::fs::read_to_string(master_path)?;

        for line in master_file.lines() {
            let mut parts = line.split(',');
            let lon = parts
                .next()
                .context("No longitude in tile.csv line")?
                .parse::<f64>()?;
            let lat = parts
                .next()
                .context("No latitude in tile.csv line")?
                .parse::<f64>()?;
            let width = parts
                .next()
                .context("No width in tile.csv line")?
                .parse::<f32>()?;
            let centre = crate::projector::LonLatCoord(geo::Coord { x: lon, y: lat });

            tiles.push(crate::tile::Tile { centre, width });
        }

        Ok(tiles)
    }

    /// Process all the tiles.
    pub async fn run_all(config: &crate::config::Atlas) -> Result<()> {
        #[expect(clippy::collapsible_if, reason = "I prefer it like this")]
        if let Some(existing_config) = crate::atlas::db::get_current_run_config().await? {
            if existing_config.run_id != config.run_id {
                color_eyre::eyre::bail!(
                    "Can't start a new run with old run data in the DB. Reset the DB."
                );
            }
        }

        let atlas = Self::new(config)?;
        let mut tile_store = super::db::worker_store().await?;

        if matches!(config.provider, crate::config::ComputeProvider::Local) {
            crate::atlas::machines::cli::new_machine(&crate::config::NewMachine {
                provider: crate::config::ComputeProvider::Local,
                ssh_key_id: "noop".to_owned(),
            })
            .await?;
        }

        tracing::debug!("Adding tile jobs to worker...");
        let start_from = crate::projector::LonLatCoord(config.centre.into());
        for master_tile in atlas.tiles.nearest_neighbor_iter(&start_from) {
            tile_store
                .push(super::tile_job::TileJob {
                    config: config.clone(),
                    tile: master_tile.data,
                })
                .await?;
        }
        tracing::debug!("...{:?}", tile_store);

        Ok(())
    }
}
