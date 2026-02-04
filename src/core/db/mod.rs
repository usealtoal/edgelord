//! Database layer for persistence using Diesel ORM.

pub mod model;
pub mod schema;

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;

use crate::error::Result;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_pool_with_memory_db() {
        let pool = create_pool(":memory:");
        assert!(pool.is_ok());
    }
}
