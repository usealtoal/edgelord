//! Persistence factory for database and recording.
//!
//! Provides factory functions for constructing database connections and
//! statistics recorders.

use std::sync::Arc;

use crate::adapter::outbound::sqlite::database::connection::{create_pool, run_migrations};
use crate::adapter::outbound::sqlite::recorder;
use crate::error::Result;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::stats::StatsRecorder;

/// Build the stats recorder backed by SQLite.
///
/// Creates a connection pool to the configured database, runs migrations,
/// and returns a stats recorder for persisting runtime statistics.
///
/// # Errors
///
/// Returns an error if:
/// - The database connection cannot be established
/// - Migrations fail to run
pub fn build_stats_recorder(config: &Config) -> Result<Arc<dyn StatsRecorder>> {
    let db_url = format!("sqlite://{}", config.database);
    let db_pool = create_pool(&db_url)?;
    run_migrations(&db_pool)?;
    Ok(recorder::create_recorder(db_pool))
}
