//! Managing Vultr resources

use color_eyre::Result;

/// Vultr state.
pub struct Machine;

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    /// Create a new Vultr machine.
    async fn create(ssh_key_id: &str) -> Result<(String, std::net::IpAddr)> {
        let ip = Self::create_machine(ssh_key_id).await?;
        Self::wait_for_machine_to_boot(ip).await?;
        Self::init_machine(ip).await?;
        Ok(("root".to_owned(), ip))
    }
}

impl Machine {
    /// Run a `vultr-cli` command.
    async fn vulr_cli(args: Vec<&str>) -> Result<String> {
        let command = super::connection::Command {
            executable: "vultr-cli".into(),
            args,
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }

    /// Create a new Vultr machine.
    async fn create_machine(ssh_key_id: &str) -> Result<std::net::IpAddr> {
        let args = vec![
            "bare-metal",
            "create",
            "--region",
            "atl",
            "--plan",
            "vbm-72c-480gb-gh200-gpu",
            "--os",
            "1743",
            "--ssh",
            ssh_key_id,
            "--label",
            "viewview-worker",
            "--notify",
            "no",
        ];

        let output = Self::vulr_cli(args).await?.trim().to_owned();
        tracing::debug!("{output}");
        //  | jq -r '.bare_metal.id'
        let ip_address = output.parse()?;
        Ok(ip_address)
    }

    /// Wait for the machine to boot.
    async fn wait_for_machine_to_boot(ip_address: std::net::IpAddr) -> Result<()> {
        let connect = format!("root@{ip_address}");
        let command = super::connection::Command {
            executable: "ssh".into(),
            args: vec!["-o", "StrictHostKeyChecking=accept-new", &connect, "true"],
            ..Default::default()
        };

        tracing::info!("Wating for machine {ip_address} to boot...");
        for _ in 0..60u8 {
            let result = super::local::Machine::command(command.clone()).await;
            if result.is_ok() {
                // Wait a bit more, in case startup scripts are running.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        color_eyre::eyre::bail!("Timed out waiting for machine {ip_address} to boot.");
    }

    /// Run all the initialisation for the machine.
    async fn init_machine(ip_address: std::net::IpAddr) -> Result<String> {
        let ip_string = ip_address.to_string();
        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["cloud_init_ubuntu22", &ip_string],
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }
}
