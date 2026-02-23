//! Application services implementing use cases for arbitrage detection and execution.
//!
//! This layer orchestrates domain logic and coordinates adapters to implement
//! the application's core workflows. Services here are responsible for:
//!
//! - Strategy detection and opportunity evaluation
//! - Risk management and position tracking
//! - Order execution and settlement handling
//! - Market relation inference for combinatorial arbitrage
//!
//! # Architecture
//!
//! The application layer follows hexagonal architecture principles:
//!
//! - Depends on domain types and port interfaces
//! - Agnostic to infrastructure concerns (HTTP, databases, etc.)
//! - Coordinates multiple adapters through port abstractions
//!
//! # Modules
//!
//! - [`cache`]: Runtime caches for order books, positions, and clusters
//! - [`cluster`]: Cluster detection for combinatorial arbitrage
//! - [`inference`]: LLM-based market relation discovery
//! - [`orchestration`]: Event processing and execution workflows
//! - [`position`]: Position lifecycle management
//! - [`risk`]: Pre-execution risk validation
//! - [`solver`]: Mathematical solvers for Bregman projection
//! - [`state`]: Shared application state and configuration
//! - [`strategy`]: Arbitrage detection algorithms

pub mod cache;
pub mod cluster;
pub mod inference;
pub mod orchestration;
pub mod position;
pub mod risk;
pub mod solver;
pub mod state;
pub mod strategy;
