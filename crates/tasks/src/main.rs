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
    pub mod daemon;
    pub mod db;
    pub mod run;
    pub mod stitch_all;
    pub mod tile_job;

    /// All the code for handling longest lines.
    pub mod longest_lines {
        pub mod grided;
        pub mod index;
        pub mod overview;
        pub mod packed;
    }
    /// Providers of compute resources.
    pub mod machines {
        pub mod cli;
        pub mod connection;
        pub mod digital_ocean;
        pub mod local;
        pub mod machine;
        pub mod new_machine_job;
        pub mod vultr;
        pub mod worker;
    }
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
    let broadcaster = apalis_board_api::sse::TracingBroadcaster::create();
    setup_logging(&broadcaster)?;

    // Safety: We're just some adhoc tasks code, so not running anywhere vulnerbale.
    unsafe {
        std::env::set_var("GDAL_NUM_THREADS", "ALL_CPUS");
    };

    let config = crate::config::Config::parse();
    tracing::info!("Initialising with config: {config:?}",);

    match &config.command {
        config::Commands::Packer(packer_config) => {
            let mut packer = packer::Packer::new(packer_config.clone())?;
            match packer_config.one {
                Some(coordinate) => packer.run_one(LonLatCoord(geo::coord! {
                    x: coordinate.0,
                    y: coordinate.1
                }))?,
                None => packer.run_all()?,
            }
        }
        config::Commands::MaxSubTiles(_) => max_subtile::run()?,
        config::Commands::Stitch(stitch_config) => {
            stitch::make_tile(
                &crate::atlas::machines::local::Machine::connection().into(),
                stitch_config,
            )
            .await?;
        }
        config::Commands::Atlas(atlas_config) => match atlas_config {
            config::AtlasCommands::Worker(worker_config) => {
                atlas::daemon::start_all(worker_config, broadcaster).await?;
            }
            config::AtlasCommands::NewMachine(new_machine_config) => {
                atlas::machines::cli::new_machine(new_machine_config).await?;
            }
            config::AtlasCommands::Run(atlas_run_config) => {
                atlas::run::Atlas::run_all(atlas_run_config).await?;
            }
            config::AtlasCommands::LongestLinesIndex(_) => {
                atlas::longest_lines::index::compile().await?;
            }
            config::AtlasCommands::LongestLinesOverviews(longest_overviews_config) => {
                atlas::longest_lines::overview::run(longest_overviews_config).await?;
            }
            config::AtlasCommands::CurrentRunConfig(_) => {
                atlas::db::print_current_run_config_as_json().await?;
            }
            config::AtlasCommands::StitchAll(stitch_all_config) => {
                atlas::stitch_all::run(stitch_all_config).await?;
            }
        },
    }

    Ok(())
}

/// Setup logging.
fn setup_logging(
    worker_broadcaster: &std::sync::Arc<
        std::sync::Mutex<apalis_board_api::sse::TracingBroadcaster>,
    >,
) -> Result<()> {
    let worker_subsriber = apalis_board_api::sse::TracingSubscriber::new(worker_broadcaster);

    let main_filters = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("info".parse()?)
        .from_env_lossy();
    let main_filter_layer = tracing_subscriber::fmt::layer().with_filter(main_filters);

    let worker_filters = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("info".parse()?)
        .from_env_lossy();
    let worker_filter_layer = worker_subsriber.layer().with_filter(worker_filters);

    tracing_subscriber::registry()
        .with(main_filter_layer)
        .with(worker_filter_layer)
        .init();

    Ok(())
}
