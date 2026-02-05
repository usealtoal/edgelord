//! Exchange component factory.
//!
//! Creates exchange-specific implementations based on configuration.

use std::sync::Arc;

use crate::app::{Config, Exchange, PolymarketConfig};
use crate::error::ConfigError;
use crate::error::Result;

use super::polymarket::{PolymarketDeduplicator, PolymarketFilter, PolymarketScorer};
use super::{
    ArbitrageExecutor, ExchangeConfig, MarketDataStream, MarketFetcher, MarketFilter,
    MarketScorer, MessageDeduplicator, OrderExecutor,
};

/// Factory for creating exchange components.
pub struct ExchangeFactory;

impl ExchangeFactory {
    fn require_polymarket_config(config: &Config) -> Result<&PolymarketConfig> {
        let poly_config = config
            .polymarket_config()
            .ok_or(ConfigError::MissingField {
                field: "polymarket_config",
            })?;

        if poly_config.ws_url.is_empty() {
            return Err(ConfigError::MissingField { field: "ws_url" }.into());
        }
        if poly_config.api_url.is_empty() {
            return Err(ConfigError::MissingField { field: "api_url" }.into());
        }

        Ok(poly_config)
    }

    /// Create a market fetcher for the configured exchange.
    pub fn create_market_fetcher(config: &Config) -> Box<dyn MarketFetcher> {
        match config.exchange {
            Exchange::Polymarket => {
                let client = config
                    .polymarket_config()
                    .map(super::polymarket::PolymarketClient::from_config)
                    .unwrap_or_else(|| {
                        super::polymarket::PolymarketClient::new(config.network().api_url.clone())
                    });
                Box::new(client)
            }
        }
    }

    /// Create a market data stream for the configured exchange.
    pub fn create_data_stream(config: &Config) -> Box<dyn MarketDataStream> {
        match config.exchange {
            Exchange::Polymarket => Box::new(super::polymarket::PolymarketDataStream::new(
                config.network().ws_url.clone(),
            )),
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
    pub async fn create_arbitrage_executor(
        config: &Config,
    ) -> Result<Option<Arc<dyn ArbitrageExecutor + Send + Sync>>> {
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
            Exchange::Polymarket => Box::new(super::polymarket::PolymarketExchangeConfig),
        }
    }

    /// Create a market scorer for the configured exchange.
    pub fn create_scorer(config: &Config) -> Result<Box<dyn MarketScorer>> {
        match config.exchange {
            Exchange::Polymarket => {
                let poly_config = Self::require_polymarket_config(config)?;
                Ok(Box::new(PolymarketScorer::new(&poly_config.scoring)))
            }
        }
    }

    /// Create a market filter for the configured exchange.
    pub fn create_filter(config: &Config) -> Result<Box<dyn MarketFilter>> {
        match config.exchange {
            Exchange::Polymarket => {
                let poly_config = Self::require_polymarket_config(config)?;
                Ok(Box::new(PolymarketFilter::new(&poly_config.market_filter)))
            }
        }
    }

    /// Create a message deduplicator for the configured exchange.
    pub fn create_deduplicator(config: &Config) -> Result<Box<dyn MessageDeduplicator>> {
        match config.exchange {
            Exchange::Polymarket => {
                let poly_config = Self::require_polymarket_config(config)?;
                Ok(Box::new(PolymarketDeduplicator::new(&poly_config.dedup)))
            }
        }
    }
}
