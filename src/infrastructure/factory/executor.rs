//! Executor factory for trade execution.
//!
//! Provides factory functions for constructing trade executors that
//! submit orders to exchanges.

use std::sync::Arc;

use tracing::{info, warn};

use crate::infrastructure::config::settings::Config;
use crate::infrastructure::exchange::factory::ExchangeFactory;
use crate::port::outbound::exchange::ArbitrageExecutor;

/// Build the trade executor if a wallet is configured.
///
/// Returns `None` if no wallet private key is configured (detection-only mode)
/// or if executor initialization fails.
pub async fn build_executor(config: &Config) -> Option<Arc<dyn ArbitrageExecutor + Send + Sync>> {
    match ExchangeFactory::create_arbitrage_executor(config).await {
        Ok(Some(exec)) => {
            info!("Executor initialized - trading ENABLED");
            Some(exec)
        }
        Ok(None) => {
            info!("No wallet configured - detection only mode");
            None
        }
        Err(e) => {
            warn!(error = %e, "Failed to initialize executor - detection only");
            None
        }
    }
}
