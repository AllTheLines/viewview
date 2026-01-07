//! Job to process a single tile.

use crate::atlas::machines::connection::Connection;
use apalis::prelude::*;
use clap::ValueEnum as _;
use color_eyre::{Result, eyre::ContextCompat as _};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

/// The size of each point in the raw DEM data.
pub const DEM_SCALE: f32 = 100.0;

/// The default location to place output files.
pub const TVS_OUTPUT_DIRECTORY: &str = "output";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// A worker job that processes a tile.
pub struct TileJob {
    /// Config from the CLI.
    pub config: crate::config::Atlas,
    /// The tile to process
    pub tile: crate::tile::Tile,
}

pub struct TileRunner<'mutex> {
    /// `mutex` is a borrowed mutex from the `Worker`. It makes sure that
    /// only a single `TileRunner` is running an L.o.S calculation.
    mutex: &'mutex Mutex<()>,

    /// Details about this particular job.
    job: TileJob,
    /// `file_prefix` is the unique-per-job file prefix that is used to store files
    /// in order to allow for unlimited concurrency
    file_prefix: String,

    /// The connection to the machine where we run the compute parts of the job.
    machine: Arc<Connection>,
}

/// `TileState` is the necessary state that a `TileJob` worker needs to coordinate
/// between other workers.
pub struct TileState {
    /// `mutex` makes sure that only one line of sight computation is happening at any given time.
    /// Any other part of the job can run at the same time, but running L.o.S is too computationally
    /// intensive to share
    /// TODO: this might not have to be a mutex, but it is too time consuming to restart the jobs
    pub mutex: Arc<Mutex<()>>,
    /// `daemon` holds an open ssh connection to the machine that all commands will be running on
    pub daemon: Arc<Connection>,
}

/// `process_tile` does the work of processing a single tile.
///
/// It is a wrapper for `TileRunner`
pub async fn process_tile(
    job: TileJob,
    state: Data<Arc<TileState>>,
) -> Result<()> {
    tracing::info!("Processing tile: {:?}", job.tile);

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();

    let runner = TileRunner {
        mutex: &state.mutex,
        file_prefix: format!("{timestamp}"),
        job,
        machine: Arc::clone(&state.daemon),
    };

    runner.run().await?;
    Ok(())
}

impl TileRunner<'_> {
    /// `run` sets up all directories, downloads necessary files, computes a L.o.S
    /// and then does post-processing and uploads
    async fn run(&self) -> Result<()> {
        self.ensure_directories().await?;

        let bt_filepath = self.download_bt_file().await?;

        self.compute(&bt_filepath).await?;

        self.assets().await?;

        if self.job.config.enable_cleanup {
            self.cleanup().await?;
        }

        tracing::debug!("Tile completed: {:?}", self.job.tile);
        Ok(())
    }

    /// Create various directories needed to process tiles.
    async fn ensure_directories(&self) -> Result<()> {
        let archive = format!("{}/output/archive", self.file_prefix);
        let longest_lines = format!("{}/output/longest_lines", self.file_prefix);

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: "mkdir".into(),
                args: vec!["-p", &archive, &longest_lines],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Delete the various files output during tile processing.
    async fn cleanup(&self) -> Result<()> {
        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: "rm".into(),
                args: vec!["-r", &self.file_prefix],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Download the packer-found, pre-stitched `.bt` DEM tile data.
    async fn download_bt_file(&self) -> Result<String> {
        let bt_filename =
            crate::stitch::canonical_filename(self.job.tile.centre.0.x, self.job.tile.centre.0.y);
        let from = format!("s3://viewview/stitched/{bt_filename}");
        let to = format!("{}/{bt_filename}", self.file_prefix);
        self.machine.sync_file_from_s3(&from, &to).await?;

        Ok(to)
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(&self, bt_filepath: &str) -> Result<()> {
        let _token = self.mutex.lock().await;

        let output = format!("{}/{}/", self.file_prefix, TVS_OUTPUT_DIRECTORY);
        let scale = format!("{DEM_SCALE}");

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: self.job.config.tvs_executable.clone(),
                args: vec![
                    "compute",
                    &bt_filepath,
                    "--output-dir",
                    &output,
                    "--scale",
                    &scale,
                    "--backend",
                    self.job
                        .config
                        .backend
                        .to_possible_value()
                        .context("Couldn't convert backend to string")?
                        .get_name(),
                    "--process",
                    "total-surfaces,longest-lines",
                    "--thread-count",
                    "48",
                ],
                env: vec![
                    ("RUST_BACKTRACE", "1"),
                    ("RUST_LOG", "off,total_viewsheds=trace"),
                ],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Process the assets needed to display the output on the website.
    async fn assets(&self) -> Result<()> {
        self.prepare_for_cloud(
            format!(
                "{}/{TVS_OUTPUT_DIRECTORY}/total_surfaces.bt",
                self.file_prefix
            )
            .as_str(),
            &self.job.tile.cog_filename()
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.s3_put_tvs_tiff().await?;
        }

        self.prepare_for_cloud(
            format!(
                "{}/{TVS_OUTPUT_DIRECTORY}/longest_lines.bt",
                self.file_prefix
            )
            .as_str(),
            &self.job.tile.cog_filename()
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.s3_put_longest_lines_cog(&self.job.tile.cog_filename())
                .await?;
        }

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Prepare a processed tile for the website UI.
    async fn prepare_for_cloud(&self, input: &str, output: &str) -> Result<()> {
        let arguments = vec!["prepare_for_cloud", input, output];

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: "./ctl.sh".into(),
                args: arguments,
                env: vec![
                    ("OUTPUT_DIR", format!("{}/output", &self.file_prefix).as_str(), ),
                ],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Sync the finished heatmap for the tile to our S3 bucket.
    async fn s3_put_tvs_tiff(&self) -> Result<()> {
        let tvs_tiff = self.job.tile.cog_filename();
        let source = format!("{}/output/archive/{tvs_tiff}", self.file_prefix);
        let destination = format!(
            "s3://viewview/runs/{}/tvs/{tvs_tiff}",
            self.job.config.run_id
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Sync a longest lines COG to our S3 bucket.
    async fn s3_put_longest_lines_cog(&self, filename: &str) -> Result<()> {
        let source_cogs = self
            .job
            .config
            .longest_lines_cogs
            .join(filename)
            .display()
            .to_string();

        let source = format!("{}/{source_cogs}", self.file_prefix);

        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/{filename}",
            self.job.config.run_id
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }
}
