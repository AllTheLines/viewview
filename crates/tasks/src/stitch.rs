//! Create arbitrary tiles from the global catalogue of DEM data.
//!
//! The tiles created will most likely have been indentified by the "Packer", also in this repo.

use color_eyre::Result;

/// A virtual DEM that represents _all_ the DEM data for the planet.
const VIRTUAL_DEM_FILE: &str = "./output/srtm.vrt";

/// How we mark points as containing no data.
const NODATA_VALUE: &str = "-32768";

/// Entrypoint.
pub fn make_tile(config: &crate::config::Stitch) -> Result<String> {
    build_virtual_dem(config)?;
    let filename = stitch(config)?;
    set_centre_as_extent(config, &filename)?;

    Ok(filename)
}

/// Build the virtual "DEM" file that represents all the DEM data for the planet. Saves having to
/// scan and parse the header for every single `.hgt` file every time we make a tile.
fn build_virtual_dem(config: &crate::config::Stitch) -> Result<()> {
    if std::path::Path::new(VIRTUAL_DEM_FILE).exists() {
        tracing::info!("Not recreating already existing {VIRTUAL_DEM_FILE}");
        return Ok(());
    }

    let mut hgts = Vec::new();
    for result in std::fs::read_dir(config.dems.clone())? {
        let file = result?.path().clone();
        if !file.is_file() {
            continue;
        }

        if let Some(extension) = file.extension()
            && extension == "hgt"
        {
            hgts.push(file.display().to_string());
        }
    }

    let mut arguments = vec![VIRTUAL_DEM_FILE];
    let mut hgts_args: Vec<&str> = hgts.iter().map(std::string::String::as_str).collect();
    tracing::info!("Adding {} `.hgt`s to {VIRTUAL_DEM_FILE}", hgts_args.len());
    arguments.append(&mut hgts_args);
    let status = std::process::Command::new("gdalbuildvrt")
        .args(arguments)
        .status()?;

    if !status.success() {
        color_eyre::eyre::bail!("Non-zero `gdal` exit status: {status}");
    }

    Ok(())
}

/// Call `gdalwarp` to construct a new stitched tile. Data will also be interpolated to metric.
fn stitch(config: &crate::config::Stitch) -> Result<String> {
    let resolution = 100.0;
    let resolution_string = resolution.to_string();
    let aeqd = format!(
        "+proj=aeqd +lat_0={} +lon_0={} +units=m +datum=WGS84 +no_defs",
        config.centre.1, config.centre.0
    );
    let output = format!("./output/{:.3},{:.3}.bt", config.centre.0, config.centre.1);

    // We align to 24 because we need to align the TVS to 8, which gives the possiblity of aligning
    // to both 4 and 8 in the SIMD algorithm.
    let align = 24.0;

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
        VIRTUAL_DEM_FILE,
        output.as_str(),
    ];
    tracing::info!("Running `gdalwarp` with args: {:?}", arguments);
    let status = std::process::Command::new("gdalwarp")
        .args(arguments)
        .status()?;
    tracing::trace!("`gdalwarp` done.");

    if !status.success() {
        color_eyre::eyre::bail!("Non-zero `gdalwarp` exit status: {status}");
    }

    Ok(output)
}

/// Re-purpose the new tile's extent header to instead define its centre.
fn set_centre_as_extent(config: &crate::config::Stitch, file: &str) -> Result<()> {
    let lon = config.centre.0.to_string();
    let lat = config.centre.1.to_string();
    let arguments = [
        "-a_ullr",
        lon.as_str(),
        lat.as_str(),
        lon.as_str(),
        lat.as_str(),
        file,
    ];
    tracing::info!("Running `gdal_edit` with args: {:?}", arguments);
    let status = std::process::Command::new("gdal_edit")
        .args(arguments)
        .status()?;

    if !status.success() {
        color_eyre::eyre::bail!("Non-zero `gdal_edit` exit status: {status}");
    }

    Ok(())
}
