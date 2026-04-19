//! An HTTP server to return viewshed data.

#![expect(
    clippy::big_endian_bytes,
    reason = "It's how we pack the viewhsed blobs"
)]

use clap::Parser as _;
use color_eyre::Result;
use tracing_subscriber::{Layer as _, layer::SubscriberExt as _, util::SubscriberInitExt as _};

mod app;
mod config;
mod get_viewshed;

#[cfg(test)]
mod test;

/// Entrypoint
#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;
    let config = crate::config::Config::parse();
    let router = app::build(config).await?;

    let address = "0.0.0.0:3333";

    tracing::info!("Starting server on: {address}");

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

/// Setup logging.
fn setup_logging() -> Result<()> {
    let filters = tracing_subscriber::EnvFilter::builder()
        .with_default_directive("server=info".parse()?)
        .from_env_lossy();
    let filter_layer = tracing_subscriber::fmt::layer().with_filter(filters);
    let tracing_setup = tracing_subscriber::registry().with(filter_layer);
    tracing_setup.init();

    Ok(())
}
