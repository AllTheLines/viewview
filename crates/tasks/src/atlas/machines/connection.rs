//! Run commands over SSH on machines.

use color_eyre::Result;

#[derive(Debug)]
/// An SSH connection to a machine.
pub struct Connection {
    /// The provider of the machine, local, digital ocean etc.
    pub provider: crate::config::ComputeProvider,
    /// An active SSH connection to the machine.
    pub ssh: Option<async_ssh2_tokio::client::Client>,
}

#[derive(Default, Debug, Clone)]
/// A command that can be run on a machine.
pub struct Command<'command> {
    /// The path of the command to execute.
    pub executable: std::path::PathBuf,
    /// The CLI arguments to the command.
    pub args: Vec<&'command str>,
    /// Environment variables to pass to the command process.
    pub env: Vec<(&'command str, &'command str)>,
    /// The directory in which to run the command
    pub current_dir: Option<std::path::PathBuf>,
}

impl Connection {
    /// Get an SSH connection to a machine.
    pub async fn connect(
        provider: crate::config::ComputeProvider,
        ip_address: std::net::IpAddr,
        user: &str,
    ) -> Result<Self> {
        if matches!(provider, crate::config::ComputeProvider::Local) {
            tracing::info!("Noop connected to {provider:?} machine.");
            return Ok(crate::atlas::machines::local::Machine::connection());
        }

        tracing::info!("Connecting to {provider:?} machine on {ip_address}");
        let auth_method = async_ssh2_tokio::AuthMethod::with_agent();

        let ssh = async_ssh2_tokio::Client::connect(
            (ip_address, 22),
            user,
            auth_method,
            async_ssh2_tokio::ServerCheckMethod::NoCheck,
        )
        .await?;

        Ok(Self {
            provider,
            ssh: Some(ssh),
        })
    }

    /// Run a command over an SSH connection.
    pub async fn command(&self, command: Command<'_>) -> Result<()> {
        tracing::debug!(
            "Running command on {:?} machine: {command:?}",
            self.provider,
        );

        if matches!(self.provider, crate::config::ComputeProvider::Local) {
            crate::atlas::machines::local::Machine::command(command).await?;
            return Ok(());
        }

        let Some(ssh) = self.ssh.as_ref() else {
            color_eyre::eyre::bail!("No SSH connection on `Connection`, this should be impossible");
        };

        let env = command
            .env
            .iter()
            .map(|env| format!("{}={}", env.0, env.1))
            .collect::<Vec<String>>()
            .join(" ");

        let current_dir = match command.current_dir {
            Some(path) => format!("/{}", path.to_string_lossy()),
            None => String::new(),
        };

        let command_string = format!(
            "cd ~/viewview{} && {} {} {}",
            current_dir,
            env,
            command.executable.display(),
            command.args.join(" ")
        );
        let output = ssh.execute(&command_string).await?;
        Self::handle_output(&command_string, output)?;

        Ok(())
    }

    /// Handle the results of the command.
    fn handle_output(
        command: &str,
        output: async_ssh2_tokio::client::CommandExecutedResult,
    ) -> Result<()> {
        let stdout = strip_ansi_escapes::strip_str(output.stdout);
        let stderr = strip_ansi_escapes::strip_str(output.stderr);
        for line in stdout.lines() {
            tracing::trace!("{line}");
        }
        for line in stderr.lines() {
            tracing::warn!("{line}");
        }

        let status = output.exit_status;

        if status != 0 {
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

    /// Sync a file to our S3 bucket.
    pub async fn sync_file_to_s3(&self, source: &str, destination: &str) -> Result<()> {
        tracing::info!(
            "Syncing file {} to {} on {:?}",
            source,
            destination,
            self.provider
        );

        let command = Command {
            executable: "./ctl.sh".into(),
            args: vec!["s3", "put", &source, &destination],
            ..Default::default()
        };

        self.command(command).await
    }

    /// Sync a file from our S3 bucket.
    pub async fn sync_file_from_s3(&self, from: &str, to: &str) -> Result<()> {
        tracing::info!("Syncing file {} from {} on {:?}", from, to, self.provider);

        let command = Command {
            executable: "./ctl.sh".into(),
            args: vec!["s3", "get", "--force", &from, &to],
            ..Default::default()
        };

        self.command(command).await
    }
}
