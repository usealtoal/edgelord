//! Exchange component factory.
//!
//! Creates exchange-specific implementations based on configuration.

use std::sync::Arc;

use crate::app::{Config, Exchange, PolymarketConfig};
use crate::error::ConfigError;
use crate::error::Result;

use super::pool::StreamFactory;

use super::polymarket::{PolymarketDeduplicator, PolymarketFilter, PolymarketScorer};
use super::{
    ArbitrageExecutor, ExchangeConfig, MarketDataStream, MarketFetcher, MarketFilter, MarketScorer,
    MessageDeduplicator, OrderExecutor,
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

    /// Create a connection pool for the configured exchange.
    ///
    /// Returns `None` if `max_connections` is 1 (use single connection instead).
    pub fn create_connection_pool(config: &Config) -> Option<super::ConnectionPool> {
        let pool_config = match &config.exchange_config {
            crate::app::ExchangeSpecificConfig::Polymarket(pm) => pm.connections.clone(),
        };

        if pool_config.max_connections <= 1 {
            return None;
        }

        let exchange_name = match config.exchange {
            Exchange::Polymarket => "polymarket",
        };

        let stream_factory = Self::create_stream_factory(config);

        Some(super::ConnectionPool::new(
            pool_config,
            config.reconnection.clone(),
            stream_factory,
            exchange_name,
        ))
    }

    /// Create a stream factory for the configured exchange.
    fn create_stream_factory(config: &Config) -> StreamFactory {
        let ws_url = config.network().ws_url.clone();
        match config.exchange {
            Exchange::Polymarket => Arc::new(move || {
                Box::new(super::polymarket::PolymarketDataStream::new(ws_url.clone()))
                    as Box<dyn MarketDataStream>
            }),
        }
    }
}
