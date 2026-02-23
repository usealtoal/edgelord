//! Test helpers for stats recorder setup.

use std::sync::Arc;

use crate::adapter::outbound::sqlite::database::connection::create_pool;
use crate::adapter::outbound::sqlite::recorder;
use crate::port::outbound::stats::StatsRecorder;

/// Build an in-memory stats recorder for tests.
pub fn in_memory_stats_recorder() -> Arc<dyn StatsRecorder> {
    let pool = create_pool("sqlite://:memory:").expect("in-memory sqlite pool should initialize");
    recorder::create_recorder(pool)
}
