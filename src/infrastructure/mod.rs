//! Infrastructure layer.
//!
//! Provides technical concerns that support the application without containing
//! business logic. This layer handles configuration, connection management,
//! and runtime resource coordination.
//!
//! # Submodules
//!
//! - [`bootstrap`] - Composition root for runtime wiring
//! - [`config`] - Configuration loading and validation
//! - [`exchange`] - Connection pooling and exchange factory
//! - [`factory`] - Component factory functions
//! - [`governor`] - Adaptive performance monitoring and scaling
//! - [`operator`] - CLI operator interface
//! - [`orchestration`] - Runtime orchestration
//! - [`subscription`] - WebSocket subscription management
//! - [`wallet`] - Wallet operations facade

pub mod bootstrap;
pub mod config;
pub mod exchange;
pub mod factory;
pub mod governor;
pub mod operator;
pub mod orchestration;
pub mod subscription;
pub mod wallet;
