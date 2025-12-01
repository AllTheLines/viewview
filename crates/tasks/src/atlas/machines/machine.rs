//! Abstracting compute resources. Could be local or a cloud VM.

use color_eyre::Result;

#[async_trait::async_trait]
/// A trait that represents a compute resource.
pub trait Machine: Sync {
    /// Instantiate
    fn new() -> Self;

    /// How to connect to the machine.
    async fn connection(&self) -> Result<Connection>;

    /// Run a command on the machine.
    async fn command(&self, command: Command<'_>) -> Result<()>;
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

#[derive(Copy, Clone)]
pub struct Connection;

/// A trait describing how to connect to a machine.
#[async_trait::async_trait]
pub trait Connect {
    /// Get the provider for the machine, eg: a local machine, a cloud machine, etc.
    fn provider(&self) -> crate::config::ComputeProvider;

    #[expect(
        dead_code,
        reason = "It's only dead because I haven't implemented any remote machines yet"
    )]
    /// Get the active connection to the machine.
    fn connection(&self) -> Connection;

    async fn connect(provider: crate::config::ComputeProvider) -> Result<Connection> {
        match provider {
            crate::config::ComputeProvider::Local => {
                super::local::Machine::new().connection().await
            }
        }
    }

    /// Return the actual machine that you can call commands with.
    fn machine(&self) -> impl Machine {
        match self.provider() {
            crate::config::ComputeProvider::Local => super::local::Machine::new(),
        }
    }
}
