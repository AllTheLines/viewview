//! A single Uber H3 grid item that contains the longest line of sight within the extent of that
//! grid item.

use color_eyre::Result;
use tokio::io::AsyncWriteExt as _;

/// The filename for the longest lines grid.
pub const LONGEST_LINES_GRIDED_FILENAME: &str = "longest_lines_grided.bin";

/// The longest line of sight in an H3 grid cell.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Grided {
    /// Longtitude
    pub lon: f32,
    /// Latitude
    pub lat: f32,
    /// The distance of the line of sight in meters.
    pub distance: u32,
}

/// Write the grided longest lines of sight to a binary file ready to be hosted on a CDN and
/// consumed by the website.
pub async fn write(path: std::path::PathBuf, world: &[Grided], run_id: &str) -> Result<()> {
    tracing::debug!("Writing `{path:?}`");
    let bytes = bytemuck::cast_slice(world);
    let mut file = tokio::fs::File::create(path.clone()).await?;
    file.write_all(bytes).await?;

    sync_to_s3(path, run_id).await?;

    Ok(())
}

/// Sync the longest lines grid to S3.
async fn sync_to_s3(source: std::path::PathBuf, run_id: &str) -> Result<()> {
    let destination =
        format!("s3://viewview/runs/{run_id}/longest_lines_cogs/{LONGEST_LINES_GRIDED_FILENAME}");
    crate::atlas::machines::local::Machine::connection()
        .sync_file_to_s3(&source.display().to_string(), &destination)
        .await?;

    Ok(())
}
