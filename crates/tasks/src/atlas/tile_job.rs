//! Job to process a single tile.

use std::sync::Arc;

use color_eyre::Result;

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

pub struct TileRunner {
    /// Details about this particular job.
    job: TileJob,

    /// The connection to the machine where we run the compute parts of the job.
    machine: Arc<crate::atlas::machines::connection::Connection>,
}

impl TileRunner {
    /// Process a single tile.
    pub async fn process(
        job: TileJob,
        state: apalis::prelude::Data<Arc<crate::atlas::daemon::State>>,
        ctx: apalis::prelude::WorkerContext,
    ) -> Result<()> {
        tracing::info!("Processing tile: {:?}", job.tile);
        let worker_name = ctx.name();
        let connections = state.connections.read().await.clone();
        let Some(connection) = connections.get(worker_name) else {
            color_eyre::eyre::bail!("No machine connection found for worker: {}", worker_name);
        };

        let runner = Self {
            job,
            machine: connection.clone(),
        };

        let bt_filepath = runner.download_bt_file().await?;
        runner.compute(&bt_filepath).await?;
        runner.assets().await?;

        tracing::debug!("Tile completed: {:?}", runner.job.tile);

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Download the packer-found, pre-stitched `.bt` DEM tile data.
    async fn download_bt_file(&self) -> Result<String> {
        let bt_filename =
            crate::stitch::canonical_filename(self.job.tile.centre.0.x, self.job.tile.centre.0.y);
        let from = format!("s3://viewview/stitched/{bt_filename}");
        let to = format!("output/{bt_filename}");
        self.machine.sync_file_from_s3(&from, &to).await?;

        Ok(to)
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(&self, bt_filepath: &str) -> Result<()> {
        let scale = format!("{DEM_SCALE}");
        self.machine
            .command(crate::atlas::machines::connection::Command {
                executable: self.job.config.tvs_executable.clone(),
                args: vec![
                    "compute",
                    &bt_filepath,
                    "--scale",
                    &scale,
                    "--backend",
                    "vulkan",
                    "--process",
                    "total-surfaces,longest-lines",
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
            format!("{TVS_OUTPUT_DIRECTORY}/total_surfaces.bt").as_str(),
            &self.job.tile.cog_filename(),
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.s3_put_tvs_tiff().await?;
        }

        self.prepare_for_cloud(
            format!("{TVS_OUTPUT_DIRECTORY}/longest_lines.bt").as_str(),
            &self.job.tile.cog_filename(),
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
                env: vec![(
                    "LONGEST_LINES_DIR",
                    &self.job.config.longest_lines_cogs.display().to_string(),
                )],
                ..Default::default()
            })
            .await?;

        Ok(())
    }

    // TODO: Make this its own job so it can be parallelised.
    /// Sync the finished heatmap for the tile to our S3 bucket.
    async fn s3_put_tvs_tiff(&self) -> Result<()> {
        let tvs_tiff = self.job.tile.cog_filename();
        let source = format!("output/archive/{tvs_tiff}");
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
        let source = self
            .job
            .config
            .longest_lines_cogs
            .join(filename)
            .display()
            .to_string();
        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/{filename}",
            self.job.config.run_id
        );

        self.machine.sync_file_to_s3(&source, &destination).await?;

        Ok(())
    }
}
