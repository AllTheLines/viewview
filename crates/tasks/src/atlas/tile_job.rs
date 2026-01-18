//! Job to process a single tile.

use crate::atlas::machines::connection::Connection;
use crate::config::RUN_ID_LOCAL;
use clap::ValueEnum as _;
use color_eyre::{Result, eyre::ContextCompat as _};
use std::sync::Arc;
use std::time::SystemTime;
use apalis::prelude::WorkerContext;
use tokio::sync::Mutex;

/// The size of each point in the raw DEM data.
pub const DEM_SCALE: f32 = 100.0;

/// The directory where all our viewview input/output goes.
pub const WORKING_DIRECTORY: &str = "work";

/// The default directory where all the longest lines COGs live.
pub const LONGEST_LINES_DIRECTORY: &str = "longest_lines";

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
    /// The unique-per-job path prefix that is used to store files
    /// in order to allow for unlimited concurrency
    job_directory: String,
    /// The connection to the machine where we run the compute parts of the job.
    machine: Arc<Connection>,
}

/// `TileState` is the necessary state that a `TileJob` worker needs to coordinate
/// between other workers.
pub struct TileWorkerState {
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
    state: apalis::prelude::Data<Arc<TileWorkerState>>,
    ctx: WorkerContext
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
    if let Err(_) = result {
        tracing::info!("shutting down worker {}", ctx.name());
        ctx.stop()?;
        return result
    }

    if cleanup {
        runner.cleanup().await?;
    }

    result
}

impl TileRunner<'_> {
    /// `run` sets up all directories, downloads necessary files, computes a L.o.S
    /// and then does post-processing and uploads
    async fn run(&self) -> Result<()> {
        self.ensure_directories().await?;

        let bt_filepath = self.download_bt_file().await?;

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
    async fn download_bt_file(&self) -> Result<String> {
        let bt_filename =
            crate::stitch::canonical_filename(self.job.tile.centre.0.x, self.job.tile.centre.0.y);
        let from = format!("s3://viewview/stitched/{bt_filename}");
        let to = format!("{}/{bt_filename}", self.job_directory);
        self.machine.sync_file_from_s3(&from, &to).await?;

        Ok(to)
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(&self, bt_filepath: &str) -> Result<()> {
        let _token = self.mutex.lock().await;

        let threads_as_string;
        let scale = format!("{DEM_SCALE}");
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
            "--scale",
            &scale,
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
        self.prepare_for_cloud(
            format!("{}/total_surfaces.bt", self.job_directory).as_str(),
            &self.job.tile.cog_filename(),
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.s3_put_raw_tvs_tiff().await?;
        }

        self.prepare_for_cloud(
            format!("{}/longest_lines.bt", self.job_directory).as_str(),
            &self.job.tile.cog_filename(),
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.s3_put_longest_lines_cog(&self.job.tile.cog_filename())
                .await?;
        }

        Ok(())
    }

    /// Prepare a processed tile for the website UI.
    async fn prepare_for_cloud(&self, input: &str, output: &str) -> Result<()> {
        let arguments = vec!["prepare_for_cloud", input, output];

        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: "./ctl.sh".into(),
                args: arguments,
                env: vec![("OUTPUT_DIR", &self.job_directory)],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    /// Sync the raw, pre-projected finished heatmap for the tile to our S3 bucket.
    ///
    /// There isn't a huge difference between this and the post-processed one, but it's a shame
    /// to have to recompute the entire planet just to get hold of this.
    async fn s3_put_raw_tvs_tiff(&self) -> Result<()> {
        let tvs_tiff = self.job.tile.cog_filename();
        let source = format!("{}/tmp/plain.tif", self.job_directory);
        let destination = format!(
            "s3://viewview/runs/{}/raw/{tvs_tiff}",
            self.job.config.run_id
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }

    /// Sync a longest lines COG to our S3 bucket.
    async fn s3_put_longest_lines_cog(&self, filename: &str) -> Result<()> {
        let source_cogs = self
            .job
            .config
            .longest_lines_cogs
            .clone()
            .unwrap_or_else(|| LONGEST_LINES_DIRECTORY.into())
            .join(filename)
            .display()
            .to_string();

        let source = format!("{}/{source_cogs}", self.job_directory);

        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/{filename}",
            self.job.config.run_id
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }
}

