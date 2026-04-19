//! Config for the CLI.

/// Config arguments.
#[derive(clap::Parser, Debug, Clone, Default)]
pub struct Config {
    /// Path to the viewshed database.
    #[arg(long, value_name = "Database path")]
    pub db_dir: std::path::PathBuf,
}
