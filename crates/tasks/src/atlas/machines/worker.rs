//! The per-machine worker that processes tiles.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use crate::atlas::machines::connection::Connection;
use crate::atlas::tile_job::{TileJob, TileState};
use apalis::layers::WorkerBuilderExt as _;
use color_eyre::Result;
use tokio::sync::Mutex;


/// `MACHINES` is a global set of IP's that are used to make sure a machine isn't
/// run more than once
static MACHINES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// machines
pub fn machines() -> &'static Mutex<HashSet<String>> {
    MACHINES.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Start a tile worker.
pub async fn tile_processor(
    job: super::new_machine_job::NewMachineJob,
    task_id: String,
) -> Result<()> {
    let tile_store =
        crate::atlas::db::atlas_worker_store::<TileJob>().await?;
    let tile_worker_name = job.tile_worker_name();

    let machine_set = machines().lock().await;

    if machine_set.contains(&tile_worker_name) {
        tracing::info!("Machine already spun up for {tile_worker_name}");
        return Ok(())
    }

    let result = Connection::connect(job.provider, job.ip_address, &job.user).await;
    let connection = match result {
        Ok(connection) => connection,
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

    drop(machine_set);

    let state = TileState {
        mutex: Arc::new(Mutex::new(())),
        daemon: Arc::new(connection),
    };


    apalis::prelude::WorkerBuilder::new(tile_worker_name)
        .backend(tile_store)
        .data(Arc::new(state))
        .concurrency(2)
        .enable_tracing()
        .build(crate::atlas::tile_job::process_tile)
        .run()
        .await?;

    Ok(())
}

