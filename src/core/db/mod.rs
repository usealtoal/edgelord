//! Database layer for persistence using Diesel ORM.

pub mod model;
pub mod schema;

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::error::Result;

/// Embedded migrations from the migrations/ directory.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Database connection pool type alias.
pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Create a connection pool for the given database URL.
///
/// # Errors
/// Returns an error if the pool cannot be created.
pub fn create_pool(database_url: &str) -> Result<DbPool> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|e| crate::error::Error::Connection(e.to_string()))
}

/// Run all pending database migrations.
///
/// # Errors
/// Returns an error if migrations fail.
pub fn run_migrations(pool: &DbPool) -> Result<()> {
    let mut conn = pool
        .get()
        .map_err(|e| crate::error::Error::Connection(e.to_string()))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| crate::error::Error::Connection(e.to_string()))?;
    Ok(())
}

/// Configure SQLite connection pragmas used for stats writes.
///
/// # Errors
/// Returns an error if a pragma fails to apply.
pub fn configure_sqlite_connection(conn: &mut SqliteConnection) -> Result<()> {
    diesel::sql_query("PRAGMA busy_timeout=5000")
        .execute(conn)
        .map_err(|e| crate::error::Error::Database(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_pool_with_memory_db() {
        let pool = create_pool(":memory:");
        assert!(pool.is_ok());
    }
}
