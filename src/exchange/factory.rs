//! Exchange component factory.
//!
//! Creates exchange-specific implementations based on configuration.

use crate::app::{Config, Exchange};
use crate::error::Result;

use super::{MarketDataStream, MarketFetcher, OrderExecutor};

/// Factory for creating exchange components.
pub struct ExchangeFactory;

impl ExchangeFactory {
    /// Create a market fetcher for the configured exchange.
    pub fn create_market_fetcher(config: &Config) -> Box<dyn MarketFetcher> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(crate::adapter::polymarket::Client::new(
                    config.network.api_url.clone(),
                ))
            }
            Exchange::Kalshi => {
                unimplemented!("Kalshi market fetcher not yet implemented")
            }
        }
    }

    /// Create a market data stream for the configured exchange.
    pub fn create_data_stream(config: &Config) -> Box<dyn MarketDataStream> {
        match config.exchange {
            Exchange::Polymarket => {
                Box::new(crate::adapter::polymarket::DataStream::new(
                    config.network.ws_url.clone(),
                ))
            }
            Exchange::Kalshi => {
                unimplemented!("Kalshi data stream not yet implemented")
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
                let executor = crate::adapter::polymarket::Executor::new(config).await?;
                Ok(Some(Box::new(executor)))
            }
            Exchange::Kalshi => {
                unimplemented!("Kalshi executor not yet implemented")
            }
        }
    }
}
