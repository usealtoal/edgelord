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
///
/// Note: Polymarket's CLOB API only supports mainnet (chain ID 137). When
/// running on testnet (chain ID 80002), executor initialization will fail
/// with an authentication error. This is expected - use dry_run mode for
/// testnet to detect opportunities without executing trades.
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
            let network = config.network();
            if network.is_testnet() {
                info!(
                    "Testnet mode - Polymarket CLOB only supports mainnet, running detection only"
                );
            } else {
                warn!(error = %e, "Failed to initialize executor - detection only");
            }
            None
        }
    }
}
