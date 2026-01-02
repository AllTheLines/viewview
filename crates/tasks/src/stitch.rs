//! Create arbitrary tiles from the global catalogue of DEM data.
//!
//! The tiles created will most likely have been indentified by the "Packer", also in this repo.

use std::sync::Arc;

use color_eyre::{Result, eyre::ContextCompat as _};

/// A virtual DEM that represents _all_ the DEM data for the planet.
const VIRTUAL_DEM_FILE: &str = "index.vrt";

/// How we mark points as containing no data.
const NODATA_VALUE: &str = "-32768";

/// Entrypoint.
pub async fn make_tile(
    machine: &Arc<crate::atlas::machines::connection::Connection>,
    config: &crate::config::Stitch,
) -> Result<String> {
    build_virtual_dem(machine, config).await?;
    let filename = stitch(machine, config).await?;
    set_centre_as_extent(machine, config, &filename).await?;

    Ok(filename)
}

/// Build the virtual "DEM" file that represents all the DEM data for the planet. Saves having to
/// scan and parse the header for every single `.hgt` file every time we make a tile.
async fn build_virtual_dem(
    machine: &Arc<crate::atlas::machines::connection::Connection>,
    config: &crate::config::Stitch,
) -> Result<()> {
    let vrt_path = std::path::Path::new(&config.dems).join(VIRTUAL_DEM_FILE);
    if vrt_path.exists() {
        tracing::info!("Not recreating already existing VRT index: {vrt_path:?}");
        return Ok(());
    }

    tracing::warn!(
        "Creating VRT index for {:?}. Don't do this on a S3 mount.",
        config.dems
    );

    let hgts = find_all_hgts(config)?;

    let vrt_path_string = vrt_path.display().to_string();
    let mut arguments = vec![vrt_path_string.as_str()];
    let mut hgts_args: Vec<&str> = hgts.iter().map(std::string::String::as_str).collect();
    tracing::info!("Adding {} `.hgt`s to {vrt_path:?}", hgts_args.len());
    arguments.append(&mut hgts_args);

    machine
        .command(crate::atlas::machines::connection::Command {
            executable: "gdalbuildvrt".into(),
            args: arguments,
            env: vec![],
            current_dir: Some(config.dems.clone()),
        })
        .await?;

    Ok(())
}

/// Find all the `*.hgt` files in the SRTM data folder.
fn find_all_hgts(config: &crate::config::Stitch) -> Result<Vec<String>> {
    let mut hgts = Vec::new();
    for result in std::fs::read_dir(config.dems.clone())? {
        let file = result?.path().clone();
        if !file.is_file() {
            continue;
        }

        if let Some(extension) = file.extension()
            && extension == "hgt"
        {
            hgts.push(
                file.file_name()
                    .context("Couldn't get `.hgt` filename")?
                    .display()
                    .to_string(),
            );
        }
    }

    Ok(hgts)
}

/// The canonical name for the stitched file. It's needed to be able to put and get the file from
/// the S3 bucket.
pub fn canonical_filename(lon: f64, lat: f64) -> String {
    format!("{lon},{lat}.bt")
}

/// Call `gdalwarp` to construct a new stitched tile. Data will also be interpolated to metric.
async fn stitch(
    machine: &Arc<crate::atlas::machines::connection::Connection>,
    config: &crate::config::Stitch,
) -> Result<String> {
    let resolution = 100.0;
    let resolution_string = resolution.to_string();
    let aeqd = format!(
        "+proj=aeqd +lat_0={} +lon_0={} +units=m +datum=WGS84 +no_defs",
        config.centre.1, config.centre.0
    );
    let output = format!(
        "./output/{}",
        canonical_filename(config.centre.0, config.centre.1)
    );
    let hgt_index = config.dems.join(VIRTUAL_DEM_FILE).display().to_string();

    // We align to 48 for the vectorising CPU kernel.
    let align = 48.0;

    let full_width_as_points = ((config.width * 3.0) / resolution).ceil();
    let full_width_aligned = (full_width_as_points / align).ceil() * align;
    let half_width = (full_width_aligned * resolution) / 2.0;
    tracing::debug!(
        "Original TVS width: {}. Aligned TVS width: {}",
        config.width,
        (half_width * 2.0) / 3.0
    );

    let min = format!("-{half_width}");
    let max = format!("{half_width}");
    let arguments = vec![
        "-overwrite",
        "-dstnodata",
        NODATA_VALUE,
        "-t_srs",
        aeqd.as_str(),
        "-te",
        min.as_str(),
        min.as_str(),
        max.as_str(),
        max.as_str(),
        "-tr",
        &resolution_string,
        &resolution_string,
        "-r",
        "bilinear",
        "-of",
        "BT",
        &hgt_index,
        output.as_str(),
    ];
    machine
        .command(crate::atlas::machines::connection::Command {
            executable: "gdalwarp".into(),
            args: arguments,
            ..Default::default()
        })
        .await?;

    Ok(output)
}

/// Re-purpose the new tile's extent header to instead define its centre.
async fn set_centre_as_extent(
    machine: &Arc<crate::atlas::machines::connection::Connection>,
    config: &crate::config::Stitch,
    file: &str,
) -> Result<()> {
    let lon = config.centre.0.to_string();
    let lat = config.centre.1.to_string();
    let arguments = vec![
        "-a_ullr",
        lon.as_str(),
        lat.as_str(),
        lon.as_str(),
        lat.as_str(),
        file,
    ];

    machine
        .command(crate::atlas::machines::connection::Command {
            executable: "gdal_edit.py".into(),
            args: arguments,
            ..Default::default()
        })
        .await?;

    Ok(())
}
