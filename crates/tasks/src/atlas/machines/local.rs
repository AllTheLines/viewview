//! A machine that represents your own local machine.

use color_eyre::Result;

/// Data for running commands on the local machine.
pub struct Machine;

#[async_trait::async_trait]
impl super::machine::Machine for Machine {
    fn new() -> Self {
        Self
    }

    async fn connection(&self) -> Result<super::machine::Connection> {
        Ok(super::machine::Connection)
    }

    async fn command(&self, command: super::machine::Command<'_>) -> color_eyre::Result<()> {
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
}
