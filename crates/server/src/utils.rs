//! Helper code

use color_eyre::Result;

/// Convert a lat/lon to a DEM coordinate.
pub fn latlon_to_dem_id(
    metadata: &crate::metadata::MetaData,
    latlon: tasks::projector::LonLatCoord,
) -> Result<u32> {
    let width_f64 = f64::from(metadata.width - 1);
    let scale = f64::from(metadata.scale);
    let coord_metric = tasks::projector::Convert {
        base: metadata.centre,
    }
    .to_meters(latlon)?;
    let offset = (width_f64 * scale) / 2.0f64;
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "The coordinates should always fit in `u32`"
    )]
    let (x, y) = {
        (
            ((coord_metric.x + offset) / scale) as u32,
            ((-coord_metric.y + offset) / scale) as u32,
        )
    };
    let dem_id = (y * metadata.width) + x;
    Ok(dem_id)
}
