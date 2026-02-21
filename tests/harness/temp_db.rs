use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use diesel::prelude::*;
use edgelord::adapters::stores::db::{create_pool, run_migrations, DbPool};

/// Temporary SQLite database for integration tests.
pub struct TempDb {
    path: PathBuf,
    pool: DbPool,
}

impl TempDb {
    pub fn create(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        path.push(format!("edgelord-{name}-{nanos}.db"));

        let url = format!("sqlite://{}", path.display());
        let pool = create_pool(&url).expect("create sqlite pool");
        run_migrations(&pool).expect("run migrations");

        // WAL mode improves concurrent writer behavior in tests.
        {
            let mut conn = pool.get().expect("get sqlite connection");
            diesel::sql_query("PRAGMA journal_mode=WAL")
                .execute(&mut conn)
                .expect("enable WAL mode");
        }

        Self { path, pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
