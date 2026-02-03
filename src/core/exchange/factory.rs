//! Exchange component factory.
//!
//! Creates exchange-specific implementations based on configuration.

use std::sync::Arc;

use crate::app::{Config, Exchange};
use crate::error::Result;

use super::{ArbitrageExecutor, ExchangeConfig, MarketDataStream, MarketFetcher, OrderExecutor};

/// Factory for creating exchange components.
pub struct ExchangeFactory;

impl ExchangeFactory {
    /// Create a market fetcher for the configured exchange.
    pub fn create_market_fetcher(config: &Config) -> Box<dyn MarketFetcher> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(super::polymarket::PolymarketClient::new(
                    config.network().api_url.clone(),
                ))
            }
        }
    }

    /// Create a market data stream for the configured exchange.
    pub fn create_data_stream(config: &Config) -> Box<dyn MarketDataStream> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(super::polymarket::PolymarketDataStream::new(
                    config.network().ws_url.clone(),
                ))
            }
        }
    }

    /// Create an order executor for the configured exchange.
    ///
    /// Returns `None` if no wallet is configured.
    pub async fn create_executor(config: &Config) -> Result<Option<Box<dyn OrderExecutor>>> {
        if config.wallet.private_key.is_none() {
            return Ok(None);
        }

        match config.exchange {
            Exchange::Polymarket => {
                let executor = super::polymarket::PolymarketExecutor::new(config).await?;
                Ok(Some(Box::new(executor)))
            }
        }
    }

    /// Create an arbitrage executor for the configured exchange.
    ///
    /// Returns `None` if no wallet is configured.
    pub async fn create_arbitrage_executor(config: &Config) -> Result<Option<Arc<dyn ArbitrageExecutor + Send + Sync>>> {
        if config.wallet.private_key.is_none() {
            return Ok(None);
        }

        match config.exchange {
            Exchange::Polymarket => {
                let executor = super::polymarket::PolymarketExecutor::new(config).await?;
                Ok(Some(Arc::new(executor)))
            }
        }
    }

    /// Create an exchange configuration for the configured exchange.
    ///
    /// Returns a boxed trait object that provides exchange-specific settings
    /// like payout amounts and outcome naming conventions.
    pub fn create_exchange_config(config: &Config) -> Box<dyn ExchangeConfig> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(super::polymarket::PolymarketExchangeConfig)
            }
        }
    }
}
