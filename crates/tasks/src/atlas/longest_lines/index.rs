//! Manage the longest lines COG files index.
//!
//! It is a text file that contains a simple list of all the `.tiff` COG files and their
//! widths in meters. This allows the web UI to easily decide whicg COG to use to look
//! for longest lines of sight.

use color_eyre::Result;

/// Update the index of all the longest lines COG files.
pub async fn compile() -> Result<()> {
    let Some(config) = crate::atlas::db::get_current_run_config().await? else {
        color_eyre::eyre::bail!("Can't save longest lines when there's no current run config.");
    };

    let tiles = crate::atlas::db::get_completed_tiles().await?;

    let index_path = config.longest_lines_cogs.join("index.txt");

    let mut index = Vec::new();
    for tile in tiles {
        let filename = tile.cog_filename();
        let line = format!(
            "{filename} {}",
            tile.width * crate::atlas::tile_job::DEM_SCALE
        );
        index.push(line);
    }

    tracing::info!(
        "Saving {} longest line COG summaries to: {:?}",
        index.len(),
        index_path
    );
    std::fs::write(index_path.clone(), index.join("\n"))?;

    if !config.is_local_run() {
        let destination = format!(
            "s3://viewview/runs/{}/longest_lines_cogs/index.txt",
            config.run_id
        );
        crate::atlas::machines::local::Machine::connection()
            .sync_file_to_s3(&index_path.display().to_string(), &destination)
            .await?;
    }

    Ok(())
}
