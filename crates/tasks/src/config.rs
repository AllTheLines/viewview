//! Defines all the CLI arguments.

use color_eyre::Result;

/// The marker to indicate that this is a local, non-production run.
const RUN_ID_LOCAL: &str = "local";

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

/// CLI subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Find a not-terrible packing of Total Viewshed tiles across the planet.
    Packer(Packer),
    /// Convert a directory of DEM data into a reduced resolution version where each point
    /// represents the highest point in its square "orbit".
    MaxSubTiles(MaxSubTiles),
    /// Create tiles identifited by the packer.
    Stitch(Stitch),

    #[command(subcommand)]
    /// Run and manage all the tasks for processing the entire planet.
    Atlas(AtlasCommands),
}

/// `atlas` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum AtlasCommands {
    /// Create a new machine
    NewMachine(NewMachine),
    /// Run
    Run(Atlas),
    /// Run and manage all the tasks for processing the entire planet.
    Worker(Worker),
    /// Create the longest lines index and sync it.
    LongestLinesIndex(LongestLinesIndex),
    /// Create overviews of longest lines..
    LongestLinesOverviews(LongestLinesOverviews),
    /// Output the current run's config.
    CurrentRunConfig(CurrentRunConfig),
    /// Stitch the entire world's `.bt` files and save them to S3.
    StitchAll(StitchAll),
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

/// `cargo run atlas stitch-all` arguments.
#[derive(clap::Parser, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StitchAll {
    /// Source of all the DEM files.
    #[arg(long, value_name = "Path to DEMs folder")]
    pub dems: std::path::PathBuf,

    /// Master tile list produced by the Packer.
    #[arg(long, value_name = "Path to master tiles list")]
    pub master: std::path::PathBuf,

    /// Number of CPUS to use,
    #[arg(long, value_name = "Number of cpus", default_value_t = number_of_cpus_on_machine())]
    pub num_cpus: usize,
}

/// Worker daemon to run Atlas jobs.
#[derive(clap::Parser, Debug, Clone)]
pub struct Worker;

/// `cargo run atlas` arguments.
#[derive(clap::Parser, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Atlas {
    /// The ID of the run. So that we can process the world without affecting the current live
    /// assets. Set to "local" for running everything locally on your machine.
    #[arg(long, value_name = "Versioned ID for run")]
    pub run_id: String,

    /// Master tile list produced by the Packer.
    #[arg(long, value_name = "Path to master tiles list")]
    pub master: std::path::PathBuf,

    /// The lon/lat coord from which to start processing
    #[arg(
        long,
        allow_hyphen_values(true),
        value_parser = parse_coord,
        value_name = "Starting coordinate")
    ]
    pub centre: (f64, f64),

    /// How many tiles to skip. Useful for resuming from the end of a previous `--amount`-based
    /// run.
    #[arg(long, value_name = "Amount of tiles to skip")]
    pub skip: Option<usize>,

    /// How many tiles to process.
    #[arg(long, value_name = "Amount of tiles to process")]
    pub amount: Option<usize>,

    /// Path to TVS executable.
    #[arg(long, value_name = "TVS executable")]
    pub tvs_executable: std::path::PathBuf,

    /// Where to save longest lines COGs.
    #[arg(long, value_name = "Longest lines COGs directory")]
    pub longest_lines_cogs: std::path::PathBuf,

    /// Where to run the computations, locally or on a cloud provider.
    #[arg(
        long,
        value_enum,
        value_name = "Compute provider",
        default_value_t = ComputeProvider::Local
    )]
    pub provider: ComputeProvider,

    /// How to run the kernel calculations.
    #[arg(
        long,
        value_enum,
        value_name = "The method of running the kernel",
        default_value_t = Backend::CPU
    )]
    pub backend: Backend,

    /// Cleanup output files after each successful tile run.
    #[arg(long)]
    pub enable_cleanup: bool,
}

/// Which kernel to run the computations on.
#[derive(clap::ValueEnum, Clone, serde::Serialize, serde::Deserialize, Debug)]
pub enum Backend {
    /// A SPIRV shader run on the GPU via Vulkan.
    Vulkan,
    /// Vulkan shader but run on the CPU.
    VulkanCPU,
    /// Optimised cache-efficient CPU kernel
    CPU,
}

/// Create a new machine for Atlas.
#[derive(clap::Parser, Debug, Clone)]
pub struct NewMachine {
    /// Where to create the machine.
    #[arg(long, value_enum, value_name = "Compute provider")]
    pub provider: ComputeProvider,

    /// The SSH key to access the new machine with. It likely already needs to be associated with
    /// your cloud account.
    #[arg(long, value_enum, value_name = "SSH key ID")]
    pub ssh_key_id: String,
}

impl Atlas {
    /// Is this a local, non-production run?
    pub fn is_local_run(&self) -> bool {
        self.run_id == RUN_ID_LOCAL
    }
}

/// Where to run the computations, locally or on a cloud provider.
#[derive(clap::ValueEnum, Clone, Debug, Copy, serde::Serialize, serde::Deserialize)]
pub enum ComputeProvider {
    /// Run everything locally.
    Local,
    /// Run on Digital Ocean compute. Requires an already authed `doctl`.
    DigitalOcean,
    /// Run on Vultr compute. Requires an already authed `vultr-ctl`.
    Vultr,
    /// Run on Google Cloud. Requires an already authed and installed `gcloud`
    GoogleCloud
}

/// Create the longest lines index and sync it.
#[derive(clap::Parser, Debug, Clone)]
pub struct LongestLinesIndex;

/// Create overviews of longest lines.
#[derive(clap::Parser, Debug, Clone)]
pub struct LongestLinesOverviews {
    /// Where longest lines COGs are saved.
    #[arg(long, value_name = "Longest lines COGs directory")]
    pub tiffs: std::path::PathBuf,

    /// The ID of the world run.
    #[arg(long, value_name = "Versioned ID for run")]
    pub run_id: String,
}

/// Get the current run's config.
#[derive(clap::Parser, Debug, Clone)]
pub struct CurrentRunConfig;

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

/// Get the number of CPUs on the machine.
pub fn number_of_cpus_on_machine() -> usize {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|section| {
            section
                .lines()
                .filter(|line| line.starts_with("processor"))
                .count()
        })
        .filter(|cpu| *cpu > 0)
        .unwrap_or(1)
}
