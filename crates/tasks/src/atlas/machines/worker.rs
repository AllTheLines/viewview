//! The per-machine worker that processes tiles.

use std::sync::Arc;

use apalis::layers::WorkerBuilderExt as _;
use color_eyre::Result;

use crate::atlas::machines::connection::Connection;

/// Start a tile worker.
pub async fn tile_processor(
    job: super::new_machine_job::NewMachineJob,
    state: apalis::prelude::Data<Arc<crate::atlas::daemon::State>>,
    task_id: String,
) -> Result<()> {
    let tile_store = crate::atlas::db::worker_store::<crate::atlas::tile_job::TileJob>().await?;
    let tile_worker_name = job.tile_worker_name();

    // We need to deref `apalis::prelude::Data<T>` so that the `T`'s type signature is saved in the
    // DB. Otherwise Apalis can't unpack the shared state.
    let derefed_state = Arc::clone(&*state);

    if !is_machine_connected(&state, &tile_worker_name).await {
        let result = Connection::connect(job.provider, job.ip_address).await;
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

        state
            .connections
            .write()
            .await
            .insert(tile_worker_name.clone(), Arc::new(connection));
    }

    apalis::prelude::WorkerBuilder::new(tile_worker_name)
        .backend(tile_store)
        .data(derefed_state)
        .concurrency(1)
        .enable_tracing()
        .build(crate::atlas::tile_job::TileRunner::process)
        .run()
        .await?;

    Ok(())
}

/// Do we have an SSH connection to the machine yet?
async fn is_machine_connected(
    state: &apalis::prelude::Data<Arc<crate::atlas::daemon::State>>,
    worker_name: &str,
) -> bool {
    state.connections.read().await.contains_key(worker_name)
}
