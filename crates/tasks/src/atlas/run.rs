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

/// The size of each point in the raw DEM data.
const DEM_SCALE: f32 = 100.0;

/// The marker to indicate that this is a local, non-production run.
const RUN_ID_LOCAL: &str = "local";

/// The default location to place output files.
pub const TVS_OUTPUT_DIRECTORY: &str = "output";

/// All the config and state needed for processing all the tiles in the world.
pub struct Atlas {
    /// Config from the CLI.
    config: crate::config::Atlas,
    /// The list of master tiles created by the Packer. Geo-indexed so we can iterate as a distance
    /// from a coordinate.
    tiles: rstar::RTree<crate::packer::TileRstar>,
    /// Status of the run.
    pub state: std::sync::Arc<tokio::sync::Mutex<super::status::Status>>,
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

        let status = super::status::Status::new(&config.status, config.run_id.clone())?;

        let atlas = Self {
            config: config.clone(),
            tiles,
            state: std::sync::Arc::new(tokio::sync::Mutex::new(status)),
        };
        Ok(atlas)
    }

    /// Get the current status of a tile.
    async fn tile_status(&self, tile: crate::tile::Tile) -> Option<super::status::State> {
        let state = &self.state.lock().await.map;
        let maybe = state.get(&tile.cog_filename());
        maybe.map(|some| some.state)
    }

    /// Update the status of a tile.
    async fn tile_update(&self, processing: super::status::Processing) -> Result<()> {
        self.state.lock().await.update(processing).await
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

    /// Is this a local, non-production run?
    fn is_local_run(&self) -> bool {
        self.config.run_id == RUN_ID_LOCAL
    }

    /// Process all the tiles.
    pub async fn run_all(config: &crate::config::Atlas) -> Result<()> {
        let atlas = Self::new(config)?;

        let start_from = crate::projector::LonLatCoord(config.centre.into());
        for master_tile in atlas.tiles.nearest_neighbor_iter(&start_from) {
            atlas.run_one(master_tile).await?;
        }

        Ok(())
    }

    /// Process a single tile.
    pub async fn run_one(&self, master_tile: &crate::packer::TileRstar) -> Result<()> {
        let status = self.tile_status(master_tile.data).await;
        if matches!(status, Some(super::status::State::Completed)) {
            // TODO: We should garbage collect crashed tile statuses. They are candidates for
            // processing too.
            tracing::debug!(
                "Skipping already completed tile: {}",
                master_tile.data.cog_filename()
            );
            return Ok(());
        }
        tracing::info!("Processing tile: {master_tile:?}");
        self.tile_update(super::status::Processing {
            tile: master_tile.data,
            state: super::status::State::Selected,
        })
        .await?;

        let bt_filepath = self.stitch(master_tile).await?;
        self.compute(master_tile, &bt_filepath).await?;
        self.assets(master_tile).await?;

        self.tile_update(super::status::Processing {
            tile: master_tile.data,
            state: super::status::State::Completed,
        })
        .await?;

        Ok(())
    }

    /// Create the custom DEM tile from the SRTM data.
    async fn stitch(&self, master_tile: &crate::packer::TileRstar) -> Result<String> {
        crate::stitch::make_tile(&crate::config::Stitch {
            dems: self.config.dems.clone(),
            centre: master_tile.data.centre.0.into(),
            width: master_tile.data.width,
        })
        .await
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(
        &self,
        master_tile: &crate::packer::TileRstar,
        bt_filepath: &str,
    ) -> Result<()> {
        self.tile_update(super::status::Processing {
            tile: master_tile.data,
            state: super::status::State::Running,
        })
        .await?;

        let scale = format!("{DEM_SCALE}");
        let arguments = vec![
            "compute",
            &bt_filepath,
            "--scale",
            &scale,
            "--backend",
            "vulkan",
            "--process",
            "total-surfaces,longest-lines",
        ];
        tracing::info!("Running `compute` with args: {:?}", arguments);
        let status = tokio::process::Command::new(&self.config.tvs_executable)
            .env("RUST_BACKTRACE", "1")
            .env("RUST_LOG", "off,total_viewsheds=trace")
            .args(arguments)
            .status()
            .await?;
        tracing::trace!("`compute` done.");

        if !status.success() {
            color_eyre::eyre::bail!("Non-zero `compute` exit status: {status}");
        }

        Ok(())
    }

    /// Process the assets needed to display the output on the website.
    async fn assets(&self, master_tile: &crate::packer::TileRstar) -> Result<()> {
        self.tile_update(super::status::Processing {
            tile: master_tile.data,
            state: super::status::State::Asseting,
        })
        .await?;
        self.prepare_for_cloud(
            format!("{TVS_OUTPUT_DIRECTORY}/total_surfaces.bt").as_str(),
            &master_tile.data.cog_filename(),
        )
        .await?;
        self.prepare_for_cloud(
            format!("{TVS_OUTPUT_DIRECTORY}/longest_lines.bt").as_str(),
            &master_tile.data.cog_filename(),
        )
        .await?;

        self.tile_update(super::status::Processing {
            tile: master_tile.data,
            state: super::status::State::ReadyForCloud,
        })
        .await?;

        if !self.is_local_run() {
            self.sync_longest_lines_cog(&master_tile.data.cog_filename())
                .await?;
            self.save_longest_lines_cogs_index().await?;
        }

        Ok(())
    }

    /// Sync a file to our S3 bucket.
    async fn sync_file_to_s3(source: &str, destination: &str) -> Result<()> {
        let arguments = vec!["s3", "put", &source, &destination];
        tracing::info!("Syncing file {} to {}", source, destination);
        let status = tokio::process::Command::new("./ctl.sh")
            .args(arguments)
            .status()
            .await?;
        tracing::trace!("Syncing done.");

        if !status.success() {
            color_eyre::eyre::bail!("Non-zero `ctl.sh` exit status: {status}");
        }

        Ok(())
    }

    // TODO:
    //
    /// Prepare a processed tile for the website UI.
    async fn prepare_for_cloud(&self, input: &str, output: &str) -> Result<()> {
        let arguments = vec!["prepare_for_cloud", input, output];
        tracing::info!(
            "Running `ctl.sh prepare_for_cloud` with args: {:?}",
            arguments
        );
        let status = tokio::process::Command::new("./ctl.sh")
            .env("LONGEST_LINES_DIR", &self.config.longest_lines_cogs)
            .args(arguments)
            .status()
            .await?;
        tracing::trace!("`ctl.sh prepare_for_cloud` done.");

        if !status.success() {
            color_eyre::eyre::bail!("Non-zero `ctl.sh prepare_for_cloud` exit status: {status}");
        }

        Ok(())
    }

    /// The canonical path to the longest lines COGS.
    fn longest_lines_cogs_index_path(&self) -> std::path::PathBuf {
        self.config.longest_lines_cogs.join("index.txt")
    }

    /// Update the index of all the longest lines COG files.
    async fn save_longest_lines_cogs_index(&self) -> Result<()> {
        let path = self.longest_lines_cogs_index_path();
        let mut tiles = Vec::new();

        #[expect(clippy::iter_over_hash_type, reason = "Ordering doesn't matter")]
        for entry in self.state.lock().await.map.values() {
            if matches!(entry.state, super::status::State::ReadyForCloud)
                || matches!(entry.state, super::status::State::Completed)
            {
                let filename = entry.key();
                let line = format!("{filename} {}", entry.tile.width * DEM_SCALE);
                tiles.push(line);
            }
        }

        tracing::info!(
            "Saving {} longest line COG summaries to: {path:?}",
            tiles.len()
        );
        std::fs::write(path, tiles.join("\n"))?;

        if !self.is_local_run() {
            self.sync_longest_lines_cogs_index().await?;
        }

        Ok(())
    }

    /// Sync a longest lines COG to our S3 bucket.
    async fn sync_longest_lines_cog(&self, filename: &str) -> Result<()> {
        let source = self
            .config
            .longest_lines_cogs
            .join(filename)
            .display()
            .to_string();
        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/{filename}",
            self.config.run_id
        );
        Self::sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }

    /// Sync the longest lines COGs index to our S3 bucket.
    async fn sync_longest_lines_cogs_index(&self) -> Result<()> {
        let source = self.longest_lines_cogs_index_path().display().to_string();
        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/index.txt",
            self.config.run_id
        );
        Self::sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }
}
