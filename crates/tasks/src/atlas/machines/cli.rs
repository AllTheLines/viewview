//! Handle machine-related CLI commands.

use crate::atlas::machines::machine::Machine as _;
use apalis::prelude::TaskSink as _;
use color_eyre::Result;

/// Add a new machine to the worker pool.
pub async fn new_machine(config: &crate::config::NewMachine) -> Result<()> {
    tracing::info!("Creating new {:?} machine...", config.provider);

    let job = match config.provider {
        crate::config::ComputeProvider::Local => {
            let ip_address =
                crate::atlas::machines::local::Machine::create(&config.ssh_key_id).await?;
            crate::atlas::machines::new_machine_job::NewMachineJob {
                ip_address,
                provider: crate::config::ComputeProvider::Local,
            }
        }

        crate::config::ComputeProvider::DigitalOcean => {
            let ip_address =
                crate::atlas::machines::digital_ocean::Machine::create(&config.ssh_key_id).await?;
            crate::atlas::machines::new_machine_job::NewMachineJob {
                ip_address,
                provider: crate::config::ComputeProvider::DigitalOcean,
            }
        }
        crate::config::ComputeProvider::Vultr => {
            let ip_address =
                crate::atlas::machines::vultr::Machine::create(&config.ssh_key_id).await?;
            crate::atlas::machines::new_machine_job::NewMachineJob {
                ip_address,
                provider: crate::config::ComputeProvider::Vultr,
            }
        }
    };

    add_new_machine_job(job.clone()).await?;

    tracing::info!(
        "{:?} machine with address {} created.",
        job.provider,
        job.ip_address
    );

    Ok(())
}

/// Add a new machine job to the new machine worker.
pub async fn add_new_machine_job(job: super::new_machine_job::NewMachineJob) -> Result<()> {
    let mut new_machine_store =
        crate::atlas::db::atlas_worker_store::<super::new_machine_job::NewMachineJob>().await?;
    new_machine_store.push(job).await?;

    Ok(())
}
