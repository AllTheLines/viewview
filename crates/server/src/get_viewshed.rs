//! Get a single viewshed from many database shards.

use color_eyre::Result;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use futures_util::StreamExt as _;
use sqlx::Row as _;

/// Return the polar segments for an entire viewshed.
pub async fn get_viewshed(
    State(state): State<crate::app::AppState>,
    Path(coordinate): Path<String>,
) -> impl IntoResponse {
    let Ok(lonlat) = shared::projector::LonLatCoord::parse(&coordinate) else {
        return (StatusCode::BAD_REQUEST, "Couldn't parse coordinate").into_response();
    };

    let Ok(dem_id) = shared::utils::lonlat_to_dem_id(&state.metadata, lonlat) else {
        return (
            StatusCode::BAD_REQUEST,
            "Couldn't get a DEM ID for the provided coordinate",
        )
            .into_response();
    };

    let start = tokio::time::Instant::now();

    let mut tasks = query_shards(state.shards, dem_id);
    let mut payload = Vec::with_capacity(1024 * 16);
    while let Some(task_result) = tasks.next().await {
        let fetch_result: Result<()> = async {
            let rows = task_result??;
            for row in rows {
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

        if let Err(error) = fetch_result {
            tracing::error!("{error:?}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Couldn't fetch viewshed data from DB",
            )
                .into_response();
        }
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

/// Query all shards in parallel.
fn query_shards(
    shards: Vec<crate::app::Shard>,
    dem_id: i64,
) -> futures_util::stream::FuturesUnordered<
    tokio::task::JoinHandle<sqlx::Result<Vec<sqlx::sqlite::SqliteRow>>>,
> {
    let tasks = futures_util::stream::FuturesUnordered::new();

    for shard in shards {
        tasks.push(tokio::spawn(async move {
            sqlx::query("SELECT angle_id, visible_segments FROM polar_segments WHERE dem_id = ?")
                .bind(dem_id)
                .fetch_all(&*shard)
                .await
        }));
    }

    tasks
}
