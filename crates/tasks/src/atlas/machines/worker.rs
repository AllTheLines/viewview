//! The per-machine worker that processes tiles.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use crate::atlas::machines::connection::Connection;
use crate::atlas::tile_job::{TileJob, TileWorkerState, WORKING_DIRECTORY};
use apalis::layers::WorkerBuilderExt as _;
use apalis::prelude::WorkerBuilder;
use color_eyre::Result;
use tokio::sync::Mutex;

/// A global set of per-machine tile workers that we use to guarantee only 1 worker is started per
/// machine.
static TILE_WORKERS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Lazy-initialised access to the tile worker set.
pub fn machines() -> &'static Mutex<HashSet<String>> {
    TILE_WORKERS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Start a tile worker.
pub async fn tile_processor(
    machine_job: super::new_machine_job::NewMachineJob,
    task_id: String,
) -> Result<()> {
    let tile_store = crate::atlas::db::atlas_worker_store::<TileJob>().await?;
    let tile_worker_name = machine_job.tile_worker_name();

    if machines().lock().await.contains(&tile_worker_name) {
        tracing::info!("Machine already spun up for {tile_worker_name}");
        return Ok(());
    }

    let result = Connection::connect(
        machine_job.provider,
        machine_job.ip_address,
        &machine_job.user,
    )
    .await;

    let connection = match result {
        Ok(connection) => {
            machines().lock().await.insert(tile_worker_name.clone());
            connection
        }
        Err(error) => {
            let message = format!("Couldn't connect to machine (job:?): {error:?}");
            tracing::error!(message);
            crate::atlas::machines::new_machine_job::set_machine_failed(
                &task_id,
                &format!("{error:?}"),
            )
            .await?;
            color_eyre::eyre::bail!(message);
        }
    };

    let state = TileWorkerState {
        mutex: Arc::new(Mutex::new(())),
        daemon: Arc::new(connection),
    };

    // clear out the previous run's state
    state.daemon
        .command(crate::atlas::machines::connection::Command {
            executable: "rm".into(),
            args: vec!["-rf", WORKING_DIRECTORY],
            ..Default::default()
        })
        .await?;

    // We allow more than one so that all tasks apart from computation can run in parallel.
    // Computation concurrency is effectively 1 due to mutex locking in the tile job.
    let worker_concurrency = 2;


    WorkerBuilder::new(tile_worker_name.clone())
        .backend(tile_store)
        .data(Arc::new(state))
        .concurrency(worker_concurrency)
        .enable_tracing()
        .build(crate::atlas::tile_job::process_tile)
        .run()
        .await?;

    machines().lock().await.remove(&tile_worker_name);

    Ok(())
}
