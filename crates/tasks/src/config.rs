//! Defines all the CLI arguments.

use color_eyre::Result;

/// `Config`
#[derive(clap::Parser, Debug)]
#[clap(author, version)]
#[command(name = "vv-tasks")]
#[command(about = "Tasks for View View")]
pub struct Config {
    #[command(subcommand)]
    /// The subcommand.
    pub command: Commands,
}

/// CLI subcommand.
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Find a not-terrible packing of Total Viewshed tiles across the planet.
    Packer(Packer),
    /// Convert a directory of DEM data into a reduced resolution version where each point
    /// represents the highest point in its square "orbit".
    MaxSubTiles(MaxSubTiles),
    /// Create tiles identifited by the packer.
    Stitch(Stitch),
}

/// `cargo run packer` arguments.
#[derive(clap::Parser, Debug, Clone)]
pub struct Packer {
    /// Just run for one step
    #[arg(
        long,
        allow_hyphen_values(true),
        value_parser = parse_coord,
        value_name = "The centre of the computation step, eg: -2.1,54.0")
    ]
    pub one: Option<(f64, f64)>,

    /// Coordinate to start the whole world from. Useful for debugging.
    #[arg(
        long,
        allow_hyphen_values(true),
        value_parser = parse_coord,
        value_name = "Starting coordinate")
    ]
    pub start: Option<(f64, f64)>,

    /// How many window steps to take Useful for debugging.
    #[arg(long, value_name = "Number of steps")]
    pub steps: Option<u32>,
}

/// `cargo run max-sub-tiles` arguments.
#[derive(clap::Parser, Debug, Clone)]
pub struct MaxSubTiles;

/// `cargo run stitch` arguments.
#[derive(clap::Parser, Debug, Clone)]
pub struct Stitch {
    /// Source of all the DEM files.
    #[arg(long, value_name = "Path to DEMs folder")]
    pub dems: std::path::PathBuf,

    /// The lon/lat coord for the centre of the tile to create.
    #[arg(
        long,
        allow_hyphen_values(true),
        value_parser = parse_coord,
        value_name = "Centre of tile")
    ]
    pub centre: (f64, f64),

    /// The width of the tile in meters.
    #[arg(long, value_name = "Tile width")]
    pub width: f32,
}

/// Parse a single coordinate.
fn parse_coord(string: &str) -> Result<(f64, f64)> {
    let mut coordinates = Vec::new();

    for coordinate in string.split(',') {
        coordinates.push(coordinate.parse::<f64>()?);
    }

    if coordinates.len() != 2 {
        color_eyre::eyre::bail!("Coordinate must be 2 numbers");
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "We already proved that the length is 2"
    )]
    Ok((coordinates[0], coordinates[1]))
}
