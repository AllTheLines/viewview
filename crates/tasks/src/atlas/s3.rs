//! Working with our S3 bucket. On Cloudflare's R2 at the time of writing

use color_eyre::Result;

/// Sync a file to our S3 bucket.
pub async fn sync_file(source: &str, destination: &str) -> Result<()> {
    tracing::info!("Syncing file {} to {}", source, destination);

    let output = tokio::process::Command::new("./ctl.sh")
        .args(vec!["s3", "put", &source, &destination])
        .output()
        .await?;

    let status = output.status;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        tracing::trace!("{line}");
    }
    for line in String::from_utf8_lossy(&output.stderr).lines() {
        tracing::trace!("{line}");
    }

    if !status.success() {
        color_eyre::eyre::bail!("Syncing file failed.");
    }

    Ok(())
}
