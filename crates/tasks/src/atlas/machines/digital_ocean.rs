//! Managing Digital Ocean resources

use color_eyre::Result;

/// Digital Ocean state.
pub struct Machine;

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    /// Create a new droplet.
    async fn create(ssh_key_id: &str) -> Result<(String, std::net::IpAddr)> {
        let ip = Self::create_droplet(ssh_key_id).await?;
        Self::wait_for_droplet_to_boot(ip).await?;
        Self::init_droplet("root", ip).await?;
        Ok(("root".to_owned(), ip))
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
        // "--image",
        // "gpu-h100x1-base",
        // "--size",
        // // "gpu-h200x1-141gb",
        // "gpu-h100x1-80gb",
        // "--region",
        // "nyc2",
        let args = vec![
            "compute",
            "droplet",
            "create",
            // ---
            "--image",
            "ubuntu-24-04-x64",
            "--size",
            // "c-48",
            "c-60-intel",
            "--region",
            "sfo2",
            "--vpc-uuid",
            "780d7b88-6d14-4cc4-85c2-950ad0f74928",
            // ---
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
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        color_eyre::eyre::bail!("Timed out waiting for machine {ip_address} to boot.");
    }

    /// Run all the initialisation for the droplet.
    async fn init_droplet(user: &str, ip_address: std::net::IpAddr) -> Result<String> {
        let connection = format!("{user}@{ip_address}");
        let command = super::connection::Command {
            executable: "./ctl.sh".into(),
            args: vec!["cloud_init_ubuntu22", &connection],
            ..Default::default()
        };
        super::local::Machine::command(command).await
    }
}
