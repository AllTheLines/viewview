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
    /// The tile to process.
    pub tile: crate::tile::Tile,
}

/// Entrypoint.
pub async fn run(config: &crate::config::StitchAll) -> Result<()> {
    let master_tiles = super::run::Atlas::load_master_tiles(&config.master)?;
    let mut stitch_store = crate::atlas::db::worker_store::<StitchJob>(STITCH_ALL_DB_PATH).await?;

    for master_tile in master_tiles {
        stitch_store
            .push(StitchJob {
                config: config.clone(),
                tile: master_tile,
            })
            .await?;
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
    let dems_path = job.config.dems.display().to_string();
    let centre = format!("{},{}", job.tile.centre.0.x, job.tile.centre.0.y);
    let width = job.tile.width.to_string();
    let tmp_directory = job.config.tmp_directory.display().to_string();
    let mut args = [
        "stitch",
        "--dems",
        &dems_path,
        "--centre",
        &centre,
        "--width",
        &width,
        "--output-dir",
        &tmp_directory,
    ]
    .iter()
    .map(std::string::ToString::to_string)
    .collect::<Vec<String>>();
    if let Some(scale) = job.config.scale {
        args.extend(vec!["--scale".to_owned(), scale.to_string()]);
    }
    let command = super::machines::connection::Command {
        executable: std::env::current_exe()?,
        args: args.iter().map(std::string::String::as_str).collect(),
        ..Default::default()
    };
    super::machines::local::Machine::command(command).await?;

    let stitch_tile_path =
        crate::stitch::canonical_filename(job.tile.centre.0.x, job.tile.centre.0.y);
    let source = format!("{tmp_directory}/{stitch_tile_path}");
    let destination = format!("s3://viewview/stitched/{stitch_tile_path}");
    let local = super::machines::local::Machine::connection();
    local.sync_file_to_s3(&source, &destination).await?;
    tracing::debug!("Removing: {source}");
    tokio::fs::remove_file(source).await?;

    Ok(())
}
