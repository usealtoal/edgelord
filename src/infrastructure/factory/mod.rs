//! Factory modules for building infrastructure components.
//!
//! Provides factory functions that construct fully-configured infrastructure
//! components from application configuration. These factories handle
//! dependency injection and wiring.
//!
//! # Submodules
//!
//! - [`executor`] - Trade executor construction
//! - [`inference`] - Relation inference service construction
//! - [`llm`] - LLM client construction
//! - [`notifier`] - Notification registry construction
//! - [`persistence`] - Database and stats recorder construction
//! - [`solver`] - Optimization solver construction
//! - [`strategy`] - Strategy registry construction

pub mod executor;
pub mod inference;
pub mod llm;
pub mod notifier;
pub mod persistence;
pub mod solver;
pub mod strategy;
