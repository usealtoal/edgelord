//! Infrastructure configuration modules.
//!
//! This module provides configuration types for all infrastructure components.
//! Configuration is typically loaded from TOML files with environment variable
//! overrides for sensitive values.
//!
//! # Submodules
//!
//! - [`cluster`] - Cluster detection service configuration
//! - [`governor`] - Adaptive subscription scaling configuration
//! - [`llm`] - LLM provider configuration for inference
//! - [`logging`] - Logging and tracing configuration
//! - [`pool`] - WebSocket connection pool configuration
//! - [`profile`] - Resource profile configuration
//! - [`risk`] - Risk management limits
//! - [`settings`] - Main application configuration
//! - [`strategy`] - Detection strategy configuration
//! - [`telegram`] - Telegram notification configuration
//! - [`wallet`] - Wallet and signing configuration

pub mod cluster;
pub mod governor;
pub mod llm;
pub mod logging;
pub mod pool;
pub mod profile;
pub mod risk;
pub mod settings;
pub mod strategy;
pub mod telegram;
pub mod wallet;
