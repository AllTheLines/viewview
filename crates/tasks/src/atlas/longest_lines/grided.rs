//! A single Uber H3 grid item that contains the longest line of sight within the extent of that
//! grid item.

use color_eyre::Result;
use tokio::io::AsyncWriteExt as _;

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
pub async fn write(path: std::path::PathBuf, world: &[Grided]) -> Result<()> {
    tracing::debug!("Writing `{path:?}`");
    let bytes = bytemuck::cast_slice(world);
    let mut file = tokio::fs::File::create(path.clone()).await?;
    file.write_all(bytes).await?;

    Ok(())
}
