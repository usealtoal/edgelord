//! Executor factory for trade execution.

use std::sync::Arc;

use tracing::{info, warn};

use crate::infrastructure::config::settings::Config;
use crate::infrastructure::exchange::factory::ExchangeFactory;
use crate::port::outbound::exchange::ArbitrageExecutor;

/// Initialize the executor if wallet is configured.
pub async fn build_executor(
    config: &Config,
) -> Option<Arc<dyn ArbitrageExecutor + Send + Sync>> {
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
