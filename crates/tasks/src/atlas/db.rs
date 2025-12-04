//! The `SQLite` database.

use color_eyre::Result;

/// The path to the `SQLite` database file.
const DB_PATH: &str = "state/atlas.db";

/// A `TileJob` as represented in the DB.
#[derive(Debug, sqlx::FromRow)]
struct TileJobRow {
    /// The JSON representation of the tile job.
    tile: sqlx::types::Json<super::tile_job::TileJob>,
}

/// A `NewMachineJob` as represented in the DB.
#[derive(Debug, sqlx::FromRow)]
pub struct NewMachineJobRow {
    /// The ID of the job.
    pub id: String,
    /// The JSON representation of the new machine job.
    pub machine: sqlx::types::Json<super::machines::new_machine_job::NewMachineJob>,
}

/// Get a conneciton to the database.
pub async fn connection() -> Result<sqlx::SqlitePool> {
    std::fs::create_dir_all("state")?;
    let options = apalis_sqlite::SqliteConnectOptions::new()
        .filename(DB_PATH)
        .create_if_missing(true);
    Ok(sqlx::SqlitePool::connect_with(options).await?)
}

/// Get the current run ID via the most recent successfully completed tile job.
pub async fn get_current_run_config() -> Result<Option<crate::config::Atlas>> {
    let db = super::db::connection().await?;
    let jobs: Vec<TileJobRow> = sqlx::query_as(include_str!("./sql/completed_tiles.sql"))
        .fetch_all(&db)
        .await?;

    let Some(job) = jobs.first() else {
        return Ok(None);
    };

    Ok(Some(job.tile.config.clone()))
}

#[expect(clippy::print_stdout, reason = "Gotta output the JSON")]
/// Get the current run ID via the most recent successfully completed tile job.
pub async fn print_current_run_config_as_json() -> Result<()> {
    let config = get_current_run_config().await?;

    let json = serde_json::to_string_pretty(&config)?;
    println!("{json}");

    Ok(())
}

/// Get the completed tiles for the current run.
pub async fn get_completed_tiles() -> Result<Vec<crate::tile::Tile>> {
    let Some(current_run_config) = get_current_run_config().await? else {
        return Ok(Vec::new());
    };

    let db = super::db::connection().await?;
    let jobs: Vec<TileJobRow> = sqlx::query_as(include_str!("./sql/completed_tiles.sql"))
        .fetch_all(&db)
        .await?;

    let mut tiles = Vec::new();

    for job in jobs {
        if job.tile.config.run_id == current_run_config.run_id {
            tiles.push(job.tile.tile);
        }
    }

    Ok(tiles)
}

/// The type returned by `worker_store()`.
pub type WorkerStore<T> = apalis_sqlite::SqliteStorage<
    T,
    apalis::prelude::json::JsonCodec<std::vec::Vec<u8>>,
    apalis_sqlite::fetcher::SqliteFetcher,
>;

/// An Apalis worker store. A store stores worker jobs. This function connects to the database and
/// creates a table for the provided type `T`.
pub async fn worker_store<T>() -> Result<WorkerStore<T>> {
    let db = super::db::connection().await?;
    apalis_sqlite::SqliteStorage::setup(&db).await?;
    let store = apalis_sqlite::SqliteStorage::<T, _, _>::new(&db);
    Ok(store)
}
