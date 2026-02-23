//! Cluster detection application services.
//!
//! Provides services for detecting arbitrage opportunities across clusters
//! of related markets using Frank-Wolfe projection.
//!
//! - [`detector`]: Core detection logic for a single cluster
//! - [`service`]: Background service monitoring order book updates

pub mod detector;
pub mod service;
