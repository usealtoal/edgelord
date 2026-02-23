//! Persistence factory for database and recording.

use std::sync::Arc;

use crate::adapter::outbound::sqlite::database::connection::{create_pool, run_migrations};
use crate::adapter::outbound::sqlite::recorder;
use crate::error::Result;
use crate::infrastructure::config::settings::Config;
use crate::port::outbound::stats::StatsRecorder;

/// Initialize SQLite and return a stats recorder.
pub fn build_stats_recorder(config: &Config) -> Result<Arc<dyn StatsRecorder>> {
    let db_url = format!("sqlite://{}", config.database);
    let db_pool = create_pool(&db_url)?;
    run_migrations(&db_pool)?;
    Ok(recorder::create_recorder(db_pool))
}
