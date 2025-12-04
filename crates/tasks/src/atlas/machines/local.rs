//! A machine that represents your own local machine.

use color_eyre::Result;

/// Data for running commands on the local machine.
pub struct Machine;

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    /// Local machines are of course already created.
    async fn create(_: &str) -> Result<std::net::IpAddr> {
        Ok(std::net::Ipv4Addr::LOCALHOST.into())
    }
}

impl Machine {
    /// A helper to return the noop "connection" to the local machine.
    pub const fn connection() -> super::connection::Connection {
        super::connection::Connection {
            provider: crate::config::ComputeProvider::Local,
            ssh: None,
        }
    }

    /// Run a local command.
    pub async fn command(command: super::connection::Command<'_>) -> color_eyre::Result<String> {
        tracing::debug!("Running local command: {command:?}");
        let mut runner = tokio::process::Command::new(&command.executable);
        runner.args(&command.args);
        for env in &command.env {
            runner.env(env.0, env.1);
        }
        if let Some(current_dir) = &command.current_dir {
            runner.current_dir(current_dir);
        }
        let output = runner
            .output()
            .await
            .map_err(|err| color_eyre::eyre::eyre!("{command:?}. {err:?}"))?;

        Self::handle_output(&command, &output)?;

        let stdout = String::from_utf8_lossy(&output.stdout).into();
        Ok(stdout)
    }

    /// Handle the results of the command.
    fn handle_output(
        command: &super::connection::Command<'_>,
        output: &std::process::Output,
    ) -> Result<()> {
        let status = output.status;
        let stdout = strip_ansi_escapes::strip_str(String::from_utf8_lossy(&output.stdout));
        let stderr = strip_ansi_escapes::strip_str(String::from_utf8_lossy(&output.stderr));
        for line in stdout.lines() {
            tracing::trace!("{line}");
        }
        for line in stderr.lines() {
            tracing::warn!("{line}");
        }

        if !status.success() {
            let message = format!(
                "{command:?}\
                \
                STDOUT:\n{stdout}\nSTDERR:\n{stderr}\n
                "
            );
            color_eyre::eyre::bail!(message);
        }

        Ok(())
    }

    /// Copy a file between the local and remote machine.
    pub async fn rsync(source: &str, destination: &str, port: u16) -> Result<String> {
        tracing::info!("Copying {source} to {destination}");
        let port_arg = format!("ssh -p {port}");
        let command = super::connection::Command {
            executable: "rsync".into(),
            args: vec!["-avP", "-e", &port_arg, &source, &destination],
            ..Default::default()
        };

        Self::command(command).await
    }
}
