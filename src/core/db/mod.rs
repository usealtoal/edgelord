//! Database layer for persistence using Diesel ORM.
//!
//! DEPRECATED: This module is being phased out. Use `crate::adapters::stores::db` instead.

// Re-export from new location for backward compatibility
pub use crate::adapters::stores::db::{
    configure_sqlite_connection, create_pool, model, run_migrations, schema, DbPool, MIGRATIONS,
};
