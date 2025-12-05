//! Setup the workers and the worker web interface.

use std::sync::Arc;

use apalis_board_api::framework::RegisterRoute as _;
use color_eyre::eyre::Result;

use apalis::{layers::WorkerBuilderExt as _, prelude::Layer as _};

/// The URL for the worker web UI.
const WEB_UI_HOST: &str = "localhost:3003";

/// I'm not 100% sure we need `dashmap` (it's a thread safe hashamp), it may be enough that the key
/// is an `Arc`. The main thing is that we don't want to read lock the entire hash, as reading
/// lasts as long as any worker is using an SSH connection. And we want to be able to add new
/// connections at any time.
type Connections = dashmap::DashMap<String, Arc<crate::atlas::machines::connection::Connection>>;

#[derive(Debug)]
/// Shared state between all workers.
pub struct State {
    /// SSH connections to machines, key by worker names.
    pub connections: tokio::sync::RwLock<Connections>,
}

/// Start the Atlas daemon
pub async fn start_all(
    _: &crate::config::Worker,
    broadcaster: std::sync::Arc<std::sync::Mutex<apalis_board_api::sse::TracingBroadcaster>>,
) -> Result<()> {
    let tile_store =
        crate::atlas::db::atlas_worker_store::<crate::atlas::tile_job::TileJob>().await?;
    let new_machine_store = crate::atlas::db::atlas_worker_store::<
        crate::atlas::machines::new_machine_job::NewMachineJob,
    >()
    .await?;

    let state = Arc::new(State {
        connections: dashmap::DashMap::new().into(),
    });

    start_existing_machines(Arc::clone(&state)).await?;

    start_web_ui(tile_store.clone(), new_machine_store.clone(), broadcaster);

    let machine_worker = apalis::prelude::WorkerBuilder::new("machines")
        .backend(new_machine_store)
        .data(state)
        .enable_tracing()
        .build(crate::atlas::machines::new_machine_job::new_machine_handler);

    tracing::info!("Starting machine worker, listening for new machines.");
    machine_worker.run().await?;

    Ok(())
}

/// Look for existing machines in the DB and open SSH connections to them.
async fn start_existing_machines(state: Arc<State>) -> Result<()> {
    let db = super::db::atlas_connection().await?;
    let jobs: Vec<crate::atlas::db::NewMachineJobRow> =
        sqlx::query_as(include_str!("./sql/active_machines.sql"))
            .fetch_all(&db)
            .await?;
    if jobs.is_empty() {
        tracing::debug!("Found 0 existing machines in the DB.");
        return Ok(());
    }

    tracing::debug!(
        "Found {} existing machines in the DB, attempting to reconnect...",
        jobs.len()
    );

    for job in jobs {
        crate::atlas::machines::new_machine_job::new_machine_handler(
            job.machine.as_ref().clone(),
            apalis::prelude::Data::new(Arc::clone(&state)),
        )
        .await?;
    }

    Ok(())
}

/// Start the Apalis job queue web UI.
fn start_web_ui(
    tile_store: super::db::WorkerStore<super::tile_job::TileJob>,
    new_machine_store: super::db::WorkerStore<super::machines::new_machine_job::NewMachineJob>,
    broadcaster: std::sync::Arc<std::sync::Mutex<apalis_board_api::sse::TracingBroadcaster>>,
) {
    tokio::spawn(async move {
        tracing::info!("Starting worker web UI on: {WEB_UI_HOST}");
        let apalis_api = apalis_board_api::framework::ApiBuilder::new(axum::Router::new())
            .register(tile_store)
            .register(new_machine_store)
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
}
