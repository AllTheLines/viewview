//! `google_cloud` is a Google Cloud

use color_eyre::eyre::Result;
use std::net::{IpAddr};
use std::time::{SystemTime};


/// Machine implements the `machine::Machine` trait for privisioning a Google Cloud machine
pub struct Machine;

impl Machine {
    /// provision runs the whole provisioning workflow for a Google Cloud machine,
    /// returning the IP address of the new machine
    async fn provision(name: &str, ssh_key_id: &str) -> Result<IpAddr> {
        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["spin_google_cloud", name, ssh_key_id],
            ..Default::default()
        };
        let res = super::local::Machine::command(command).await?;
        let ip = res.trim();

        Ok(ip.parse()?)
    }

    /// `wait_to_machine_to_boot` makes sure that the `ip_address` is not in your known hosts
    /// before booting into the machine and ensuring an ssh connection is possible.
    ///
    /// Google Cloud frequently hands out the same IP to different runs creating
    /// issues with host checking. Instead of infecting all other ssh commands with key checking,
    /// it is much easier just to clear that IP out from `known_hosts`
    async fn wait_for_machine_to_boot(ip_address: IpAddr) -> Result<()> {
        let ip_string = ip_address.to_string();

        let delete_ip = super::connection::Command {
            executable: "ssh-keygen".into(),
            args: vec!["-f", "/home/ryan/.ssh/known_hosts", "-R", ip_string.as_str()],
            ..Default::default()
        };
        super::local::Machine::command(delete_ip.clone()).await?;

        let connect = format!("atlas@{ip_string}");
        let command = super::connection::Command {
            executable: "ssh".into(),
            args: vec!["-o", "StrictHostKeyChecking=accept-new", &connect, "true"],
            ..Default::default()
        };

        tracing::info!("Wating for machine {ip_string} to boot...");
        for _ in 0..60u8 {
            let result = super::local::Machine::command(command.clone()).await;
            if result.is_ok() {
                // Wait a bit more, in case DO startup scripts are running.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        color_eyre::eyre::bail!("Timed out waiting for machine {ip_string} to boot.");
    }

    /// `init` provisions a `GoogleCloud` Debian machine with the Ubuntu startup script
    async fn init(ip_address: IpAddr) -> Result<String> {
        let connect = format!("atlas@{ip_address}");

        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["cloud_init_ubuntu22", &connect],
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }
}

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    async fn create(ssh_key_id: &str) -> Result<(String, IpAddr)> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis();
        let machine_name = format!("atlas-{timestamp}");

        let ip = Self::provision(&machine_name, ssh_key_id).await?;

        Self::wait_for_machine_to_boot(ip).await?;

        Self::init(ip).await?;

        Ok(("atlas".to_owned(), ip))
    }
}
