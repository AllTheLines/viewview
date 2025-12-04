//! An Apalis worker job to handle new machines.

use std::sync::Arc;

use color_eyre::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// A job to handle new machines.
pub struct NewMachineJob {
    /// The new machine's IP address.
    pub ip_address: std::net::IpAddr,
    /// The provider of this machine, local, digital ocean, etc.
    pub provider: crate::config::ComputeProvider,
}

impl NewMachineJob {
    /// Canonical key for a tile worker. Allows matching a worker to a machine's SSH connection.
    pub fn tile_worker_name(&self) -> String {
        format!("tiler-{:?}-{}", self.provider, self.ip_address)
    }
}

/// The callback to run when a new machine job is added.
pub async fn new_machine_handler(
    job: super::new_machine_job::NewMachineJob,
    state: apalis::prelude::Data<Arc<crate::atlas::daemon::State>>,
) -> Result<()> {
    tracing::trace!("Forwarding state to worker: {state:?}");
    let machine_task_id = get_machine_task_id_hack(job.clone()).await?;

    tokio::spawn(async move {
        let result = super::worker::tile_processor(job, state, machine_task_id).await;
        if let Err(error) = result {
            tracing::error!("Error starting tile worker for new machine: {error:?}");
        }
    });
    Ok(())
}

/// Get the internal ID of the job. Just a hack until this is fixed:
///   <https://github.com/apalis-dev/apalis/issues/645>
async fn get_machine_task_id_hack(
    job_to_find: super::new_machine_job::NewMachineJob,
) -> Result<String> {
    let db = crate::atlas::db::connection().await?;
    let jobs: Vec<crate::atlas::db::NewMachineJobRow> = sqlx::query_as(
        "
            SELECT
              id,
              CAST(job AS TEXT) as machine
            FROM Jobs
            WHERE job_type = 'tasks::atlas::machines::new_machine_job::NewMachineJob'
            ORDER BY done_at DESC;
        ",
    )
    .fetch_all(&db)
    .await?;

    for job in jobs {
        if job.machine.ip_address == job_to_find.ip_address {
            return Ok(job.id);
        }
    }

    color_eyre::eyre::bail!("Couldn't find job {job_to_find:?} in DB.");
}

/// Manually set a new machine job to failed. We most likely do this when we try to connect to it
/// over SSH and it fails.
pub async fn set_machine_failed(id: &str, error: &str) -> Result<()> {
    let db = crate::atlas::db::connection().await?;
    sqlx::query(include_str!("../sql/fail_machine_job.sql"))
        .bind(error)
        .bind(id)
        .execute(&db)
        .await?;
    Ok(())
}
