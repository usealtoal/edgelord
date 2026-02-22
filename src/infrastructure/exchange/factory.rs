//! Exchange component factory.
//!
//! Creates exchange-specific implementations based on configuration.

use std::sync::Arc;

use crate::adapter::outbound::polymarket::client::PolymarketClient;
use crate::adapter::outbound::polymarket::dedup::PolymarketDeduplicator;
use crate::adapter::outbound::polymarket::executor::PolymarketExecutor;
use crate::adapter::outbound::polymarket::filter::PolymarketFilter;
use crate::adapter::outbound::polymarket::market::PolymarketMarketParser;
use crate::adapter::outbound::polymarket::scorer::PolymarketScorer;
use crate::adapter::outbound::polymarket::settings::{PolymarketConfig, PolymarketRuntimeConfig};
use crate::adapter::outbound::polymarket::stream::PolymarketDataStream;
use crate::error::ConfigError;
use crate::error::Result;
use crate::infrastructure::config::settings::{Config, Exchange};

use super::pool::ConnectionPool;
use super::pool::StreamFactory;

use crate::port::outbound::dedup::MessageDeduplicator;
use crate::port::outbound::exchange::{
    ArbitrageExecutor, MarketDataStream, MarketFetcher, MarketParser, OrderExecutor,
};
use crate::port::outbound::filter::{MarketFilter, MarketScorer};

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

    fn polymarket_runtime_config(config: &Config) -> Result<PolymarketRuntimeConfig> {
        let private_key =
            config
                .wallet
                .private_key
                .as_ref()
                .cloned()
                .ok_or(ConfigError::MissingField {
                    field: "WALLET_PRIVATE_KEY",
                })?;
        let network = config.network();
        Ok(PolymarketRuntimeConfig {
            private_key,
            chain_id: network.chain_id,
            api_url: network.api_url,
            environment: network.environment,
        })
    }

    /// Create a market fetcher for the configured exchange.
    pub fn create_market_fetcher(config: &Config) -> Box<dyn MarketFetcher> {
        match config.exchange {
            Exchange::Polymarket => {
                let client = config
                    .polymarket_config()
                    .map(PolymarketClient::from_config)
                    .unwrap_or_else(|| PolymarketClient::new(config.network().api_url.clone()));
                Box::new(client)
            }
        }
    }

    /// Create a market data stream for the configured exchange.
    pub fn create_data_stream(config: &Config) -> Box<dyn MarketDataStream> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(PolymarketDataStream::new(config.network().ws_url.clone()))
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
                let runtime = Self::polymarket_runtime_config(config)?;
                let executor = PolymarketExecutor::new(&runtime).await?;
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
                let runtime = Self::polymarket_runtime_config(config)?;
                let executor = PolymarketExecutor::new(&runtime).await?;
                Ok(Some(Arc::new(executor)))
            }
        }
    }

    /// Create a market parser for the configured exchange.
    ///
    /// Returns a boxed trait object that parses exchange payloads into domain
    /// markets using exchange-specific conventions.
    pub fn create_market_parser(config: &Config) -> Box<dyn MarketParser> {
        match config.exchange {
            Exchange::Polymarket => Box::new(PolymarketMarketParser),
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
    /// Returns an error if the pool configuration is invalid.
    pub fn create_connection_pool(config: &Config) -> Result<Option<ConnectionPool>> {
        let pool_config = match &config.exchange_config {
            crate::infrastructure::config::settings::ExchangeSpecificConfig::Polymarket(pm) => {
                crate::infrastructure::config::pool::ConnectionPoolConfig {
                    max_connections: pm.connections.max_connections,
                    subscriptions_per_connection: pm.connections.subscriptions_per_connection,
                    connection_ttl_secs: pm.connections.connection_ttl_secs,
                    preemptive_reconnect_secs: pm.connections.preemptive_reconnect_secs,
                    health_check_interval_secs: pm.connections.health_check_interval_secs,
                    max_silent_secs: pm.connections.max_silent_secs,
                    channel_capacity: pm.connections.channel_capacity,
                }
            }
        };

        if pool_config.max_connections <= 1 {
            return Ok(None);
        }

        let exchange_name = match config.exchange {
            Exchange::Polymarket => "polymarket",
        };

        let stream_factory = Self::create_stream_factory(config);

        let pool = ConnectionPool::new(
            pool_config,
            config.reconnection.clone(),
            stream_factory,
            exchange_name,
        )?;

        Ok(Some(pool))
    }

    /// Create a stream factory for the configured exchange.
    fn create_stream_factory(config: &Config) -> StreamFactory {
        let ws_url = config.network().ws_url.clone();
        match config.exchange {
            Exchange::Polymarket => Arc::new(move || {
                Box::new(PolymarketDataStream::new(ws_url.clone())) as Box<dyn MarketDataStream>
            }),
        }
    }
}
