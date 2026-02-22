//! Infrastructure layer.
//!
//! Technical concerns that support the application:
//!
//! - `config` - Configuration loading and validation
//! - `exchange` - Connection pooling and factory
//! - `governor` - Adaptive performance monitoring and scaling
//! - `subscription` - WebSocket subscription management

pub mod bootstrap;
pub mod config;
pub mod exchange;
pub mod governor;
pub mod operator;
pub mod orchestration;
pub mod subscription;
pub mod wallet;
