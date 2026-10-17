//! Abstracting compute resources. Could be local or a cloud VM.

use color_eyre::Result;

#[expect(
    clippy::double_must_use,
    reason = "Do we even need async_trait anymore?"
)]
#[async_trait::async_trait]
/// A trait that represents a compute resource.
pub trait Machine: Sync + Send {
    /// Create a machine
    async fn create(ssh_key_id: &str) -> Result<(String, std::net::IpAddr)>
    where
        Self: Sized;
}
