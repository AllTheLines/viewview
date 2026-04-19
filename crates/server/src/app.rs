//! The main app code.

use std::sync::Arc;

use color_eyre::{Result, eyre::ContextCompat as _};

use axum::{Router, routing};

use sqlx::ConnectOptions as _;

/// Shorthand for a single connection to a single shard. Even though it's a `Pool` it's limited to
/// a concurrency of 1.
pub type Shard = Arc<sqlx::Pool<sqlx::Sqlite>>;

/// All state needed for the app's lifetime.
#[derive(Clone)]
pub struct AppState {
    /// A collection of single-concurrency pools, one for each database shard.
    pub shards: Vec<Shard>,
    /// Metadata about the underlying DEM that generated the viewsheds.
    pub metadata: shared::metadata::MetaData,
}

/// Entrypoint for starting the server.
pub async fn build(config: crate::config::Config) -> Result<axum::Router> {
    let pools = crate::app::all_databases(config).await?;
    crate::app::router(pools).await
}

/// Setup all the databases.
pub async fn all_databases(config: crate::config::Config) -> Result<Vec<Shard>> {
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "It's too verbose"
    )]
    let db_paths: Vec<_> = std::fs::read_dir(&config.db_dir)?
        .filter_map(|result| result.ok())
        .map(|file| file.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "db"))
        .collect();

    let mut pools = Vec::new();
    for path in db_paths {
        let pool = one_database(&path).await?;
        pools.push(Arc::new(pool));
    }

    tracing::info!("Connected to {} databases.", pools.len());

    Ok(pools)
}

/// Setup a DB.
async fn one_database(path: &std::path::PathBuf) -> Result<sqlx::Pool<sqlx::Sqlite>> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Off)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Off)
        .disable_statement_logging()
        .read_only(true)
        .immutable(true)
        .pragma("locking_mode", "EXCLUSIVE")
        .pragma("cache_size", "-20000")
        .pragma("mmap_size", "1073741824")
        .pragma("query_only", "ON");

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// Server routes.
async fn router(shards: Vec<Shard>) -> Result<Router> {
    let metadata = load_metadata(
        shards
            .first()
            .context("No DBs available to query metadate from.")?,
    )
    .await?;
    let state = AppState { shards, metadata };

    let router = Router::new()
        .route("/", routing::get(|| async { "hello" }))
        .route(
            "/viewshed/{coordinate}",
            routing::get(crate::get_viewshed::get_viewshed),
        )
        .with_state(state)
        .layer(
            tower_http::compression::CompressionLayer::new()
                .gzip(true)
                .br(true),
        )
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin([
                    "http://localhost:3000".parse()?,
                    "https://alltheviews.world".parse()?,
                    "https://map.alltheviews.world".parse()?,
                    "https://galiano.alltheviews.world".parse()?,
                    "https://tombh-galiano-viewview.tom-364.workers.dev".parse()?,
                ])
                .allow_methods([axum::http::Method::GET]),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http().on_response(
            |_response: &axum::response::Response,
             latency: std::time::Duration,
             _span: &tracing::Span| {
                tracing::debug!("Request completed in {:?}", latency);
            },
        ));

    Ok(router)
}

/// Load the metadata for the DEM that generated the viewsheds.
async fn load_metadata(shard: &sqlx::Pool<sqlx::Sqlite>) -> Result<shared::metadata::MetaData> {
    let (json,): (String,) = sqlx::query_as("SELECT json FROM metadata")
        .fetch_one(shard)
        .await?;

    let metadata: shared::metadata::MetaData = serde_json::from_str(&json)?;
    tracing::info!("Loaded metadata: {metadata:?}");
    Ok(metadata)
}
