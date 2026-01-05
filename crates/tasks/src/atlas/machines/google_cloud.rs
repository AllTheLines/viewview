use std::fmt::format;
use color_eyre::eyre::Result;
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Instant, SystemTime};

pub struct Machine;

impl Machine {

    async fn provision(name: &str, ssh_key_id: &str) -> Result<IpAddr> {
        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["spin_google_cloud", name, ssh_key_id],
            ..Default::default()
        };
        let res = super::local::Machine::command(command).await?;
        let ip = res.trim();

        println!("{:#?}", ip);

        Ok(ip.parse()?)
    }

    async fn wait_for_machine_to_boot(ip_address: IpAddr) -> Result<()> {
        let connect = format!("atlas@{ip_address}");
        let command = super::connection::Command {
            executable: "ssh".into(),
            args: vec!["-o", "StrictHostKeyChecking=accept-new", &connect, "true"],
            ..Default::default()
        };

        tracing::info!("Wating for machine {ip_address} to boot...");
        for _ in 0..60u8 {
            let result = super::local::Machine::command(command.clone()).await;
            if result.is_ok() {
                // Wait a bit more, in case DO startup scripts are running.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        color_eyre::eyre::bail!("Timed out waiting for machine {ip_address} to boot.");
    }

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
    async fn create(ssh_key_id: &str) -> Result<IpAddr> {
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
        let machine_name = format!("atlas-{}", timestamp);

        let ip = Self::provision(&machine_name, ssh_key_id).await?;

        Self::wait_for_machine_to_boot(ip).await?;

        Self::init(ip).await?;

        Ok(ip)
    }
}
