//! The main app code.

use color_eyre::Result;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};

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
    let options = sqlx::sqlite::SqliteConnectOptions::new().filename(config.db_path);
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
        );

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

    let result: std::result::Result<Vec<(u16, Vec<u8>)>, sqlx::Error> = sqlx::query_as(
        "
        SELECT angle_id, MIN(visible_segments)
        FROM polar_segments
        WHERE dem_id = ?1
        GROUP BY angle_id;
        ",
    )
    .bind(dem_id)
    .fetch_all(&state.pool)
    .await;

    let Ok(rows) = result else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Couldn't fetch viewshed data from DB",
        )
            .into_response();
    };

    if rows.is_empty() {
        let msg = format!("No viewshed found. Using DEM ID: {dem_id}");
        return (StatusCode::NOT_FOUND, msg).into_response();
    }

    if rows.len() > 360 {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "More than 360 rows found",
        )
            .into_response();
    }

    // Framing: for each blob: `u16` angle + `u16` length + bytes
    let mut payload = Vec::new();

    for blob in &rows {
        payload.extend(blob.0.to_be_bytes());
        #[expect(
            clippy::as_conversions,
            clippy::cast_possible_truncation,
            reason = "The number of segments for a given angle must fit in `u16`"
        )]
        payload.extend((blob.1.len() as u16).to_be_bytes());
        payload.extend(blob.1.clone());
    }

    let body = bytes::Bytes::from(payload);
    tracing::debug!(
        "Viewshed request for DEM ID {dem_id} found {} bytes",
        body.len()
    );
    (
        StatusCode::OK,
        [("content-type", "application/octet-stream")],
        body,
    )
        .into_response()
}
