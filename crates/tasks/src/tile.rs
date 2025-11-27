//! A single computable tile for the Total Viewshed algorithm.

use color_eyre::{Result, eyre::ContextCompat as _};
use geo::{Area as _, BoundingRect as _, Buffer as _};
use rstar::PointDistance as _;

use crate::projector::LonLatCoord;

/// The tile data itself.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tile {
    /// The centre of the tile.
    pub centre: crate::projector::LonLatCoord,
    /// The width of the tile. Therefore, not the distance from the centre to an edge. Width is
    /// better than radius because it defines the minimum line of sight distance we are interested
    /// in.
    pub width: f32,
}

impl Tile {
    /// Convert the width to the shortest distance between the centre and an edge.
    fn radius(&self) -> f64 {
        f64::from(self.width / 2.0)
    }

    /// The centre coordinate reprojected to the given metric projection.
    pub fn centre_metric(&self, anchor: crate::projector::LonLatCoord) -> Result<geo::Coord> {
        let projecter = crate::projector::Convert { base: anchor };
        projecter.to_meters(self.centre)
    }

    /// The Axis-Aligned Bounding Box for the tile. Note that this has an unintuitive shape as
    /// "axis-alighed" in lon/lat coordinates are very much not circles nor squares near the poles.
    /// Nevertheless the AABB is still useful for quicker first pass lookups of containing points.
    /// A follow up `is_within()` for each found point can then be used to get the exact contents.
    pub fn to_aabb_lonlat(self) -> Result<rstar::AABB<LonLatCoord>> {
        let bbox = self
            .to_polygon_lonlat()
            .bounding_rect()
            .context(format!("Couldn't find bbox for tile: {self:?}"))?;
        let aabb = rstar::AABB::from_corners(LonLatCoord(bbox.min()), LonLatCoord(bbox.max()));

        Ok(aabb)
    }

    /// Make a polygon representing the tile in metric coordinates.
    pub fn to_polygon_metric(
        self,
        anchor: crate::projector::LonLatCoord,
    ) -> Result<geo::MultiPolygon> {
        let centre = self.centre_metric(anchor)?;
        let circle = geo::Point::new(centre.x, centre.y).buffer(self.radius());
        Ok(circle)
    }

    /// Make a polygon representing the tile in lon/lat coordinates.
    pub fn to_polygon_lonlat(self) -> geo::Polygon<f64> {
        let resolution: u16 = 360;
        let mut coordinates = Vec::with_capacity((resolution + 1).into());

        for i in 0..=resolution {
            let angle = f64::from(i) * 360.0f64 / f64::from(resolution);
            let centre = geo::Point::new(self.centre.0.x, self.centre.0.y);
            let destination = Self::destination(centre, angle, self.radius());
            coordinates.push((destination.x(), destination.y()));
        }

        let circle = geo::LineString::from(coordinates);
        geo::Polygon::new(circle, vec![])
    }

    /// The surface area covered by the tile.
    pub fn surface_area(self) -> Result<f32> {
        let polygon = self.to_polygon_metric(self.centre)?;
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "Is there another way?"
        )]
        Ok(polygon.unsigned_area() as f32)
    }

    /// Calculate the distance in meters of the tile from the given point.
    pub fn distance_from(&self, point_lonlat: LonLatCoord) -> Result<f64> {
        let projector = crate::projector::Convert { base: point_lonlat };
        let point = projector.to_meters(self.centre)?;

        Ok(self.centre_metric(point_lonlat)?.distance_2(&point).sqrt())
    }

    #[expect(
        clippy::suboptimal_flops,
        reason = "I copied it from the `geo` crate and daren't mess with it."
    )]
    /// Find the lon/lat coordinate of a point based in its distance and bearing from another
    /// point.
    fn destination(origin: geo::Point, bearing: f64, meters: f64) -> geo::Point {
        let center_lng = origin.x().to_radians();
        let center_lat = origin.y().to_radians();
        let bearing_rad = bearing.to_radians();

        let rad = meters / f64::from(crate::projector::EARTH_RADIUS * 1000.0);

        let lat =
            { center_lat.sin() * rad.cos() + center_lat.cos() * rad.sin() * bearing_rad.cos() }
                .asin();
        let lng = { bearing_rad.sin() * rad.sin() * center_lat.cos() }
            .atan2(rad.cos() - center_lat.sin() * lat.sin())
            + center_lng;

        geo::Point::new(lng.to_degrees(), lat.to_degrees())
    }

    /// Canonical filename for the tile.
    pub fn cog_filename(&self) -> String {
        format!("{}_{}.tiff", self.centre.0.x, self.centre.0.y)
    }
}
