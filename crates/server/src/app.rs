//! The main app code.

use color_eyre::Result;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};

use futures_util::StreamExt as _;
use sqlx::ConnectOptions as _;
use sqlx::Row as _;

/// All state needed for the app's lifetime.
#[derive(Clone)]
struct AppState {
    /// Pool of connections to the DB.
    pool: sqlx::Pool<sqlx::Sqlite>,
    /// Metadata about the underlying DEM that generated the viewsheds.
    metadata: crate::metadata::MetaData,
}

/// Setup the DB.
pub async fn db(config: crate::config::Config) -> Result<sqlx::Pool<sqlx::Sqlite>> {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(config.db_path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Off)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Off)
        .disable_statement_logging()
        .read_only(true)
        .immutable(true)
        .pragma("locking_mode", "EXCLUSIVE")
        .pragma("cache_size", "-20000")
        .pragma("mmap_size", "1073741824")
        .pragma("query_only", "ON");
    let pool = sqlx::sqlite::SqlitePool::connect_with(options).await?;

    Ok(pool)
}

/// Server routes.
pub async fn router(pool: sqlx::Pool<sqlx::Sqlite>) -> Result<Router> {
    let metadata = load_metadata(&pool).await?;
    let state = AppState { pool, metadata };

    let router = Router::new()
        .route("/", routing::get(|| async { "hello" }))
        .route("/viewshed/{coordinate}", routing::get(get_viewshed))
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
async fn load_metadata(pool: &sqlx::Pool<sqlx::Sqlite>) -> Result<crate::metadata::MetaData> {
    let (json,): (String,) = sqlx::query_as("SELECT json FROM metadata")
        .fetch_one(pool)
        .await?;

    let metadata: crate::metadata::MetaData = serde_json::from_str(&json)?;
    tracing::info!("Loaded metadata: {metadata:?}");
    Ok(metadata)
}

/// Return the polar segments for an entire viewshed.
async fn get_viewshed(
    State(state): State<AppState>,
    Path(coordinate): Path<String>,
) -> impl IntoResponse {
    let Ok(lonlat) = tasks::projector::LonLatCoord::parse(&coordinate) else {
        return (StatusCode::BAD_REQUEST, "Couldn't parse coordinate").into_response();
    };

    let Ok(dem_id) = crate::utils::latlon_to_dem_id(&state.metadata, lonlat) else {
        return (
            StatusCode::BAD_REQUEST,
            "Couldn't get a DEM ID for the provided coordinate",
        )
            .into_response();
    };

    let start = tokio::time::Instant::now();
    let mut rows =
        sqlx::query("SELECT angle_id, visible_segments FROM polar_segments WHERE dem_id = ?1")
            .bind(dem_id)
            .fetch(&state.pool);

    let mut payload = Vec::with_capacity(1024 * 16);
    let result: Result<()> = async {
        // Streaming the results should be faster.
        while let Some(row_result) = rows.next().await {
            let row = row_result?;

            let angle_id: u16 = row.try_get(0)?;
            let bytes: Vec<u8> = row.try_get(1)?;
            let bytes_length = u16::try_from(bytes.len())?;

            payload.extend_from_slice(&angle_id.to_be_bytes());
            payload.extend_from_slice(&bytes_length.to_be_bytes());
            payload.extend_from_slice(&bytes);
        }
        Ok(())
    }
    .await;

    if result.is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't fetch viewshed data from DB",
        )
            .into_response();
    }

    if payload.is_empty() {
        let msg = format!("No viewshed found. Using DEM ID: {dem_id}");
        return (StatusCode::NOT_FOUND, msg).into_response();
    }

    let body = bytes::Bytes::from(payload);
    tracing::debug!(
        "Viewshed request for DEM ID {dem_id} found {} bytes in {:?}",
        body.len(),
        start.elapsed()
    );
    (
        StatusCode::OK,
        [("content-type", "application/octet-stream")],
        body,
    )
        .into_response()
}
