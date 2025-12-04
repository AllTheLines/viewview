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

        let bt_filepath = runner.stitch().await?;
        runner.compute(&bt_filepath).await?;
        runner.assets().await?;

        tracing::debug!("Tile completed: {:?}", runner.job.tile);

        Ok(())
    }

    /// Create the custom DEM tile from the SRTM data.
    async fn stitch(&self) -> Result<String> {
        let local_machine = Arc::new(super::machines::local::Machine::connection());
        let bt_filepath = crate::stitch::make_tile(
            &local_machine,
            &crate::config::Stitch {
                dems: self.job.config.dems.clone(),
                centre: self.job.tile.centre.0.into(),
                width: self.job.tile.width,
            },
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.copy_stitched_bt_to_remote(&bt_filepath).await?;
        }

        Ok(bt_filepath)
    }

    /// Copy the stitched `.bt` file to the remote machine.
    async fn copy_stitched_bt_to_remote(&self, bt_filepath: &str) -> Result<()> {
        let (address, port) = self.machine.get_rsync_details();
        let destination_path = format!("{address}:/root/viewview/{bt_filepath}");

        tracing::debug!(
            "Uploading {bt_filepath} to {:?} {}",
            self.machine.provider,
            destination_path
        );

        crate::atlas::machines::local::Machine::rsync(bt_filepath, &destination_path, port).await?;

        Ok(())
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
            self.download_tvs_tiff().await?;
        }

        self.prepare_for_cloud(
            format!("{TVS_OUTPUT_DIRECTORY}/longest_lines.bt").as_str(),
            &self.job.tile.cog_filename(),
        )
        .await?;

        if !self.job.config.is_local_run() {
            self.sync_longest_lines_cog(&self.job.tile.cog_filename())
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
    /// Download the finished heatmap for the tile.
    async fn download_tvs_tiff(&self) -> Result<()> {
        let (address, port) = self.machine.get_rsync_details();
        let heatmap_tiff = self.job.tile.cog_filename();
        let heatmap_path = format!("{address}:/root/viewview/output/archive/{heatmap_tiff}");
        let archive_directory = std::path::Path::new("output")
            .join("archive")
            .join(self.job.config.run_id.clone());
        tokio::fs::create_dir_all(archive_directory.clone()).await?;
        let destination_path = archive_directory.join(&heatmap_tiff).display().to_string();

        tracing::debug!(
            "Downloading {heatmap_path:?} from {:?} to {:?}",
            self.machine.provider,
            destination_path
        );

        crate::atlas::machines::local::Machine::rsync(&heatmap_path, &destination_path, port)
            .await?;

        Ok(())
    }

    /// Sync a longest lines COG to our S3 bucket.
    async fn sync_longest_lines_cog(&self, filename: &str) -> Result<()> {
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
