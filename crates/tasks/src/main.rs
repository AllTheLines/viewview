//! Entrypoint
#![expect(
    clippy::panic_in_result_fn,
    reason = "This is just code for short tasks, so panicking is better"
)]
#![cfg_attr(
    test,
    expect(
        clippy::indexing_slicing,
        clippy::as_conversions,
        clippy::unreadable_literal,
        reason = "Tests aren't so strict"
    )
)]

/// The code for processing the entire world.
mod atlas {
    pub mod run;
    pub mod status;
}

mod config;
mod max_subtile;
mod packer;
mod projector;
mod stitch;
mod tile;

use clap::Parser as _;
use color_eyre::Result;
use tracing_subscriber::{Layer as _, layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::projector::LonLatCoord;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;

    // Safety: We're just some adhoc tasks code, so not running anywhere vulnerbale.
    unsafe {
        std::env::set_var("GDAL_NUM_THREADS", "ALL_CPUS");
    };

    let config = crate::config::Config::parse();
    tracing::info!("Initialising with config: {config:?}",);

    match &config.command {
        crate::config::Commands::Packer(packer_config) => {
            let mut packer = packer::Packer::new(packer_config.clone())?;
            match packer_config.one {
                Some(coordinate) => packer.run_one(LonLatCoord(geo::coord! {
                    x: coordinate.0,
                    y: coordinate.1
                }))?,
                None => packer.run_all()?,
            }
        }
        crate::config::Commands::MaxSubTiles(_) => max_subtile::run()?,
        crate::config::Commands::Stitch(stitch_config) => {
            stitch::make_tile(stitch_config).await?;
        }
        crate::config::Commands::Atlas(atlas_config) => {
            atlas::run::Atlas::run_all(atlas_config).await?;
        }
    }

    Ok(())
}

/// Setup logging.
fn setup_logging() -> Result<()> {
    let filters = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("info".parse()?)
        .from_env_lossy();
    let filter_layer = tracing_subscriber::fmt::layer().with_filter(filters);
    let tracing_setup = tracing_subscriber::registry().with(filter_layer);
    tracing_setup.init();

    Ok(())
}
