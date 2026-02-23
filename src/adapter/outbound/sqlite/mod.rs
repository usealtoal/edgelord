//! SQLite persistence adapters.
//!
//! Provides SQLite-backed implementations for statistics recording,
//! relation storage, and report generation using Diesel ORM.

pub mod database;
pub mod recorder;
pub mod report;
pub mod store;
