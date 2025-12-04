//! Managing Digital Ocean resources

use color_eyre::Result;

/// Digital Ocean state.
pub struct Machine;

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    /// Create a new droplet.
    async fn create(ssh_key_id: &str) -> Result<std::net::IpAddr> {
        let ip = Self::create_droplet(ssh_key_id).await?;
        Self::wait_for_droplet_to_boot(ip).await?;
        Self::init_droplet(ip).await?;
        Ok(ip)
    }
}

impl Machine {
    /// Run a `doctl` command.
    async fn doctl(args: Vec<&str>) -> Result<String> {
        let command = super::connection::Command {
            executable: "doctl".into(),
            args,
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }

    /// Create a new droplet.
    async fn create_droplet(ssh_key_id: &str) -> Result<std::net::IpAddr> {
        // A cheap Ubuntu machine for testing
        // let args = vec![
        //     "compute",
        //     "droplet",
        //     "create",
        //     "--image",
        //     "ubuntu-24-04-x64",
        //     "--size",
        //     "s-1vcpu-2gb-70gb-intel",
        //     "--region",
        //     "sfo2",
        //     "--vpc-uuid",
        //     "780d7b88-6d14-4cc4-85c2-950ad0f74928",
        //     "viewview-worker",
        //     "--tag-name",
        //     "viewview",
        //     "--ssh-keys",
        //     ssh_key_id,
        //     "--format",
        //     "PublicIPv4",
        //     "--no-header",
        //     "--wait",
        // ];

        let args = vec![
            "compute",
            "droplet",
            "create",
            "--image",
            "gpu-h100x1-base",
            "--size",
            "gpu-h200x1-141gb",
            "--region",
            "nyc2",
            "--enable-monitoring",
            "--tag-names",
            "viewview",
            "--ssh-keys",
            ssh_key_id,
            "viewview-worker",
            "--format",
            "PublicIPv4",
            "--no-header",
            "--wait",
        ];

        let output = Self::doctl(args).await?.trim().to_owned();
        let ip_address = output.parse()?;
        Ok(ip_address)
    }

    /// Wait for the droplet to boot.
    async fn wait_for_droplet_to_boot(ip_address: std::net::IpAddr) -> Result<()> {
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
                // Wait a bit more, in case DO startup scripts are running.
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        color_eyre::eyre::bail!("Timed out waiting for machine {ip_address} to boot.");
    }

    /// Run all the initialisation for the droplet.
    async fn init_droplet(ip_address: std::net::IpAddr) -> Result<String> {
        let ip_string = ip_address.to_string();
        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["init_digital_ocean", &ip_string],
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }
}
