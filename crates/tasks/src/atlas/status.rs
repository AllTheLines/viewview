//! Managing the status of world runs.

use std::collections::HashMap;

use color_eyre::Result;
use tokio::io::AsyncWriteExt as _;

#[derive(Default, serde::Serialize, serde::Deserialize)]
/// Status of all completed and running tiles.
pub struct Status {
    #[serde(skip)]
    /// Where the status JSON is saved.
    path: std::path::PathBuf,
    /// The ID of the run. So that we can process the world without affecting the current live
    /// assets.
    run_id: String,
    /// Processing/processed tiles.
    pub map: std::collections::HashMap<String, Processing>,
}

impl Status {
    /// Instantiate
    pub fn new(path: &std::path::PathBuf, run_id: String) -> Result<Self> {
        let processing = Self::load(path, run_id)?;
        Ok(processing)
    }

    /// Load the status file.
    fn load(path: &std::path::PathBuf, run_id: String) -> Result<Self> {
        if !path.exists() {
            tracing::warn!("No status file found at `{:?}`, starting fresh.", path);
            let status = Self {
                path: path.clone(),
                run_id,
                map: HashMap::default(),
            };

            return Ok(status);
        }

        let contents = std::fs::read_to_string(path)?;
        let mut status: Self = serde_json::from_str(&contents)?;
        status.path.clone_from(path);

        if status.run_id != run_id {
            color_eyre::eyre::bail!(
                "Run ID {} in {path:?} does not match CLI run ID: {run_id}",
                status.run_id
            );
        }

        Ok(status)
    }

    /// Update the status file
    pub async fn update(&mut self, updated: Processing) -> Result<()> {
        self.map.insert(updated.key(), updated);
        tracing::trace!(
            "Saving status `{:?}` for {} to {:?}",
            updated.state,
            updated.tile.cog_filename(),
            self.path
        );
        self.save().await
    }

    /// Save the status file.
    async fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        let mut file = tokio::fs::File::create(self.path.clone()).await?;
        file.write_all(json.as_bytes()).await?;

        Ok(())
    }
}

/// A processing tile.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Processing {
    /// The details of the processing/completed tile.
    pub tile: crate::tile::Tile,
    /// The processing state of the tile.
    pub state: State,
}

impl Processing {
    /// The canonical hash key for a tile.
    pub fn key(&self) -> String {
        self.tile.cog_filename()
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
/// The various states in the processing of a single tile.
pub enum State {
    /// The tile has been selected for processing.
    Selected,
    /// The tile is being processed for viewsheds.
    Running,
    /// The total surfaces heatmap and the longest lines COG are being generated and copied to
    /// buckets.
    Asseting,
    /// Assets are ready to be synced to S3.
    ReadyForCloud,
    /// The tile has been processed and assets created.
    Completed,
    /// The tile errored.
    Errored,
}
