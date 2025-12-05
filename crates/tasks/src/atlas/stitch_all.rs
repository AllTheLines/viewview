//! Stitch the entire world's `.bt` files and save them to S3.

/// The stitcher has its own database separate from Atlas.
const STITCH_ALL_DB_PATH: &str = "state/stitch_all.db";

use apalis::{layers::WorkerBuilderExt as _, prelude::TaskSink as _};
use color_eyre::Result;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
/// A worker job that processes a tile.
pub struct StitchJob {
    /// Config from the CLI.
    pub config: crate::config::StitchAll,
    /// The tile to process
    pub tile: crate::tile::Tile,
}

/// Entrypoint.
pub async fn run(config: &crate::config::StitchAll) -> Result<()> {
    if let Some(master) = config.clone().master
        && config.clone().dems.is_some()
    {
        let master_tiles = super::run::Atlas::load_master_tiles(&master)?;
        let mut stitch_store =
            crate::atlas::db::worker_store::<StitchJob>(STITCH_ALL_DB_PATH).await?;

        for master_tile in master_tiles.iter().skip(500) {
            stitch_store
                .push(StitchJob {
                    config: config.clone(),
                    tile: *master_tile,
                })
                .await?;
        }
    }

    daemon(config.num_cpus).await?;

    Ok(())
}

/// Start the stitcher Apalis workers.
async fn daemon(num_cpus: usize) -> Result<()> {
    let stitch_store = crate::atlas::db::worker_store::<StitchJob>(STITCH_ALL_DB_PATH).await?;

    let machine_worker = apalis::prelude::WorkerBuilder::new("stitcher")
        .backend(stitch_store)
        .concurrency(num_cpus)
        .enable_tracing()
        .build(process);

    tracing::info!("Starting stitcher workers...");
    machine_worker.run().await?;

    Ok(())
}

/// Process a single tile.
pub async fn process(job: StitchJob) -> Result<()> {
    let Some(dems) = job.config.dems else {
        color_eyre::eyre::bail!(
            "--dems must be set in the original command that created the jobs."
        );
    };
    let dems_path = dems.display().to_string();
    let centre = format!("{},{}", job.tile.centre.0.x, job.tile.centre.0.y);
    let width = job.tile.width.to_string();
    let command = super::machines::connection::Command {
        executable: "target/debug/tasks".into(),
        args: vec![
            "stitch", "--dems", &dems_path, "--centre", &centre, "--width", &width,
        ],
        ..Default::default()
    };
    super::machines::local::Machine::command(command).await?;

    let stitch_tile_path = format!("{centre}.bt");
    let source = format!("output/{stitch_tile_path}");
    let destination = format!("s3://viewview/stitched/{stitch_tile_path}");
    let local = super::machines::local::Machine::connection();
    local.sync_file_to_s3(&source, &destination).await?;

    Ok(())
}
