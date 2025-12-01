//! Setup the workers and the worker web interface.

use apalis_board_api::framework::RegisterRoute as _;
use color_eyre::eyre::Result;

use apalis::{layers::WorkerBuilderExt as _, prelude::Layer as _};

/// The URL for the worker web UI.
const WEB_UI_HOST: &str = "localhost:3003";

/// The type returned by `worker_store()`.
type WorkerStore<T> = apalis_sqlite::SqliteStorage<
    T,
    apalis::prelude::json::JsonCodec<std::vec::Vec<u8>>,
    apalis_sqlite::fetcher::SqliteFetcher,
>;

/// An Apalis worker store. A store stores worker jobs. This function connects to the database and
/// creates a table for the provided type `T`.
pub async fn worker_store<T>() -> Result<WorkerStore<T>> {
    let db = super::db::connection().await?;
    apalis_sqlite::SqliteStorage::setup(&db).await?;
    let store = apalis_sqlite::SqliteStorage::<T, _, _>::new(&db);
    Ok(store)
}

/// Placeholder until we have remote machines.
async fn machiner(details: String) -> Result<()> {
    tracing::info!("Machine: {details}");
    Ok(())
}

/// Start the worker daemon
pub async fn daemon(
    _: &crate::config::Worker,
    broadcaster: std::sync::Arc<std::sync::Mutex<apalis_board_api::sse::TracingBroadcaster>>,
) -> Result<()> {
    let tile_store = worker_store::<crate::atlas::tile_job::TileJob>().await?;
    let machine_store = worker_store::<String>().await?;

    // machine_store.push("TBD".to_owned()).await?;

    let tile_worker = apalis::prelude::WorkerBuilder::new("tiles")
        .backend(tile_store.clone())
        // TODO: set concurrency to the number of machines available in the pool.
        .concurrency(1)
        .enable_tracing()
        .build(super::tile_job::TileRunner::process);

    let machine_worker = apalis::prelude::WorkerBuilder::new("machines")
        .backend(machine_store)
        .enable_tracing()
        .build(machiner);

    tokio::spawn(async move {
        tracing::info!("Starting worker web UI on: {WEB_UI_HOST}");
        let apalis_api = apalis_board_api::framework::ApiBuilder::new(axum::Router::new())
            .register(tile_store)
            .build();
        let layer = tower_http::normalize_path::NormalizePathLayer::trim_trailing_slash();
        let router = axum::Router::new()
            .nest("/api/v1", apalis_api)
            .fallback_service(apalis_board_api::ui::ServeUI::new())
            .layer(axum::Extension(broadcaster));

        let result = async {
            let listener = tokio::net::TcpListener::bind(WEB_UI_HOST).await?;
            let axum_app =
                axum::ServiceExt::<axum::extract::Request>::into_make_service(layer.layer(router));
            axum::serve(listener, axum_app).await
        }
        .await;

        if let Err(error) = result {
            tracing::error!("Web UI error: {error:?}");
        }
    });

    tracing::info!("Starting worker daemons: tiles, machines");
    tokio::try_join!(tile_worker.run(), machine_worker.run())?;

    Ok(())
}
