//! Job to process a single tile.

use color_eyre::Result;

use crate::atlas::machines::machine::{Connect as _, Machine as _};

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

    #[expect(
        dead_code,
        reason = "It's only dead because I haven't implemented any remote machines yet"
    )]
    /// The connection to the machine where we run the compute parts of the job.
    machine: super::machines::machine::Connection,
}

impl TileRunner {
    /// Process a single tile.
    pub async fn process(job: TileJob) -> Result<()> {
        tracing::info!("Processing tile: {:?}", job.tile);
        let provider = job.config.provider;
        let runner = Self {
            job,
            machine: Self::connect(provider).await?,
        };

        let bt_filepath = runner.stitch().await?;
        runner.compute(&bt_filepath).await?;
        runner.assets().await?;

        tracing::debug!("Tile completed: {:?}", runner.job.tile);
        Ok(())
    }

    /// Create the custom DEM tile from the SRTM data.
    async fn stitch(&self) -> Result<String> {
        crate::stitch::make_tile(
            &self.machine(),
            &crate::config::Stitch {
                dems: self.job.config.dems.clone(),
                centre: self.job.tile.centre.0.into(),
                width: self.job.tile.width,
            },
        )
        .await
    }

    /// Run the TVS kernel on a single tile.
    async fn compute(&self, bt_filepath: &str) -> Result<()> {
        let scale = format!("{DEM_SCALE}");
        self.machine()
            .command(crate::atlas::machines::machine::Command {
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
        self.machine()
            .command(crate::atlas::machines::machine::Command {
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

        super::s3::sync_file(&source, &destination).await?;

        Ok(())
    }
}

impl super::machines::machine::Connect for TileRunner {
    fn provider(&self) -> crate::config::ComputeProvider {
        self.job.config.provider
    }

    fn connection(&self) -> super::machines::machine::Connection {
        self.machine
    }
}
