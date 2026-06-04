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

    let neighbourhood_id = get_neighbourhood_id(
        dem_id,
        state.metadata.width.into(),
        state.metadata.neighbourhood_size.into(),
    );

    let start = tokio::time::Instant::now();

    let mut tasks = query_shards(state.shards, neighbourhood_id);
    let mut payload = Vec::with_capacity(1024 * 16);
    payload.extend_from_slice(&state.metadata.angle_subdivisions.to_be_bytes());
    let mut is_dem_id_of_biggest_added = false;
    while let Some(task_result) = tasks.next().await {
        let fetch_result: Result<()> = async {
            let rows = task_result??;
            for row in rows {
                let angle_id: u16 = row.try_get(0)?;
                let dem_id_of_biggest_viewshed: i64 = row.try_get(1)?;
                let bytes: Vec<u8> = row.try_get(2)?;
                let bytes_length = u16::try_from(bytes.len())?;

                // TODO: This should all be done via another table. It'll save a fair bit of storage
                // space too.
                if !is_dem_id_of_biggest_added {
                    let lonlat_of_biggest_viewshed = shared::utils::dem_id_to_lonlat(
                        &state.metadata,
                        dem_id_of_biggest_viewshed,
                    )?;
                    tracing::debug!("Lon/lat of biggest viewshed: {lonlat_of_biggest_viewshed:?}");
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::as_conversions,
                        reason = "Just lonlat coordinates"
                    )]
                    {
                        payload.extend_from_slice(
                            &(lonlat_of_biggest_viewshed.0.x as f32).to_be_bytes(),
                        );
                        payload.extend_from_slice(
                            &(lonlat_of_biggest_viewshed.0.y as f32).to_be_bytes(),
                        );
                    };
                    is_dem_id_of_biggest_added = true;
                }

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

/// Find which neighbourhood a given index is in.
pub const fn get_neighbourhood_id(index: i64, global_width: i64, neighbourhood_size: i64) -> i64 {
    let global_x = index.rem_euclid(global_width);
    let global_y = index.div_euclid(global_width);

    let neighbourhood_width = neighbourhood_size.isqrt();
    let neighbourhoods_per_row = global_width.div_euclid(neighbourhood_width);
    let neighbourhood_x = global_x.div_euclid(neighbourhood_width);
    let neighbourhood_y = global_y.div_euclid(neighbourhood_width);
    (neighbourhood_y * neighbourhoods_per_row) + neighbourhood_x
}

/// Query all shards in parallel.
fn query_shards(
    shards: Vec<crate::app::Shard>,
    neighbourhood_id: i64,
) -> futures_util::stream::FuturesUnordered<
    tokio::task::JoinHandle<sqlx::Result<Vec<sqlx::sqlite::SqliteRow>>>,
> {
    let tasks = futures_util::stream::FuturesUnordered::new();

    for shard in shards {
        tasks.push(tokio::spawn(async move {
            sqlx::query(
                "
                SELECT angle_id, dem_id, visible_segments
                FROM polar_segments
                WHERE neighbourhood_id = ?
                ",
            )
            .bind(neighbourhood_id)
            .fetch_all(&*shard)
            .await
        }));
    }

    tasks
}
