//! Helper code.

use color_eyre::Result;

#[inline]
/// Convert a lon/lat to a DEM coordinate.
///
/// # Errors
///
/// If projection errors.
pub fn lonlat_to_dem_id(
    metadata: &crate::metadata::MetaData,
    latlon: crate::projector::LonLatCoord,
) -> Result<i64> {
    let width_f64 = f64::from(metadata.width + 1);
    let scale = f64::from(metadata.scale);
    let coord_metric = crate::projector::Convert {
        base: metadata.centre,
    }
    .to_meters(latlon)?;
    let offset = (width_f64 * scale) / 2.0f64;
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        reason = "The coordinates should always fit in `u64`"
    )]
    let (x, y) = {
        (
            ((coord_metric.x + offset) / scale) as i64,
            ((-coord_metric.y + offset) / scale) as i64,
        )
    };
    let dem_id = (y * i64::from(metadata.width)) + x;
    Ok(dem_id)
}
