//! Job to process a single tile.

use crate::atlas::machines::connection::Connection;
use crate::config::RUN_ID_LOCAL;
use apalis::prelude::WorkerContext;
use clap::ValueEnum as _;
use color_eyre::{Result, eyre::ContextCompat as _};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

/// The directory where all our viewview input/output goes.
pub const WORKING_DIRECTORY: &str = "work";

/// Filename for longest lines COG.
const LONGEST_LINES_COG: &str = "longest_lines.cog.tiff";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// A worker job that processes a tile.
pub struct TileJob {
    /// Config from the CLI.
    pub config: crate::config::Atlas,
    /// The tile to process.
    pub tile: crate::tile::Tile,
}

/// Data for running a single tile job.
pub struct TileRunner<'mutex> {
    /// `mutex` is a borrowed mutex from the `Worker`. It makes sure that
    /// only a single `TileRunner` is running an L.o.S calculation.
    mutex: &'mutex Mutex<()>,
    /// Details about this particular job.
    job: TileJob,
    /// The unique-per-job path prefix that is used to store files
    /// in order to allow for unlimited concurrency.
    job_directory: String,
    /// The connection to the machine where we run the compute parts of the job.
    machine: Arc<Connection>,
}

/// Necessary state that a `TileJob` worker needs to coordinate between other workers.
pub struct TileWorkerState {
    /// `mutex` makes sure that only one line of sight computation is happening at any given time.
    /// Any other part of the job can run at the same time, but running L.o.S is too computationally
    /// intensive to share
    /// TODO: this might not have to be a mutex, but it is too time consuming to restart the jobs.
    pub mutex: Arc<Mutex<()>>,
    /// `daemon` holds an open ssh connection to the machine that all commands will be running on.
    pub daemon: Arc<Connection>,
}

/// `process_tile` does the work of processing a single tile.
///
/// It is a wrapper for `TileRunner`.
pub async fn process_tile(
    job: TileJob,
    state: apalis::prelude::Data<Arc<TileWorkerState>>,
    ctx: WorkerContext,
) -> Result<()> {
    tracing::info!("Processing tile: {:?}", job.tile);

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis()
        .to_string();

    let job_id = if job.config.run_id == RUN_ID_LOCAL {
        "local".to_owned()
    } else {
        timestamp
    };

    let cleanup = job.config.enable_cleanup;

    let runner = TileRunner {
        mutex: &state.mutex,
        job_directory: format!("{WORKING_DIRECTORY}/{job_id}"),
        job,
        machine: Arc::clone(&state.daemon),
    };

    let result = runner.run().await;
    if result.is_err() {
        tracing::info!("shutting down worker {}", ctx.name());
        ctx.stop()?;
        return result;
    }

    if cleanup {
        runner.cleanup().await?;
    }

    result
}

impl TileRunner<'_> {
    /// `run` sets up all directories, downloads necessary files, computes a L.o.S
    /// and then does post-processing and uploads.
    async fn run(&self) -> Result<()> {
        self.ensure_directories().await?;

        let bt_filepath = self.download_stitched_geotiff().await?;

        self.compute(&bt_filepath).await?;

        self.assets().await?;

        tracing::debug!("Tile completed: {:?}", self.job.tile);
        Ok(())
    }

    /// Create various directories needed to process tiles.
    async fn ensure_directories(&self) -> Result<()> {
        let archive = format!("{}/archive", self.job_directory);
        let longest_lines = format!("{}/longest_lines", self.job_directory);

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
                args: vec!["-r", &self.job_directory],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Download the packer-found, pre-stitched `.bt` DEM tile data.
    async fn download_stitched_geotiff(&self) -> Result<String> {
        let filename =
            crate::stitch::canonical_filename(self.job.tile.centre.0.x, self.job.tile.centre.0.y);
        let from = format!("s3://viewview/stitched/{filename}");
        let to = format!("{}/{filename}", self.job_directory);
        self.machine.sync_file_from_s3(&from, &to).await?;

        Ok(to)
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(&self, bt_filepath: &str) -> Result<()> {
        let _token = self.mutex.lock().await;

        let threads_as_string;
        let backend = self
            .job
            .config
            .backend
            .to_possible_value()
            .context("Couldn't convert backend to string")?;

        let mut args = vec![
            "compute",
            &bt_filepath,
            "--output-dir",
            &self.job_directory,
            "--disable-image-render",
            "--backend",
            backend.get_name(),
            "--process",
            "total-surfaces,longest-lines",
        ];

        if let Some(threads) = self.job.config.cpu_kernel_threads {
            threads_as_string = threads.to_string();
            args.extend(["--thread-count", &threads_as_string]);
        }

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: self.job.config.tvs_executable.clone(),
                args,
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
        self.make_longest_lines_cog().await?;

        if !self.job.config.is_local_run() {
            self.s3_put_longest_lines_cog().await?;
            self.s3_put_raw_tvs_tiff().await?;
        }

        Ok(())
    }

    /// It needs to be a COG because it's queried from the browser.
    async fn make_longest_lines_cog(&self) -> Result<()> {
        let plain_tif = format!("{}/longest_lines.tiff", self.job_directory);
        let cog = format!("{}/{}", self.job_directory, LONGEST_LINES_COG);

        let args = vec![
            "-of",
            "COG",
            // Smallest valid size (cos we're just querying for single raster points)
            "-co",
            "BLOCKSIZE=128",
            "-co",
            "RESAMPLING=NEAREST",
            "-co",
            "OVERVIEWS=NONE",
            "-co",
            "COMPRESS=DEFLATE",
            "-co",
            "PREDICTOR=3",
            &plain_tif,
            &cog,
        ];

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: "/usr/bin/gdal_translate".into(),
                args,
                env: vec![],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Sync the raw, pre-projected finished heatmap for the tile to our S3 bucket.
    ///
    /// It's tempting to do the full post-processing here, but it's a lossy step, so any mistake
    /// and we can't get back the data.
    async fn s3_put_raw_tvs_tiff(&self) -> Result<()> {
        let source = format!("{}/total_surfaces.tiff", self.job_directory);
        let destination = format!(
            "s3://viewview/runs/{}/raw/{}",
            self.job.config.run_id,
            self.job.tile.canonical_filename()
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }

    /// Sync a longest lines COG to our S3 bucket.
    /// It needs to be a COG because it's queried from the browser.
    async fn s3_put_longest_lines_cog(&self) -> Result<()> {
        let cog = format!("{}/{}", self.job_directory, LONGEST_LINES_COG);
        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/{}",
            self.job.config.run_id,
            self.job.tile.canonical_filename()
        );

        self.machine.sync_file_to_s3(&cog, &destination).await?;

        Ok(())
    }
}
