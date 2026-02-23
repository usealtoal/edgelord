//! Market stream setup.

use crate::domain::id::TokenId;
use crate::error::Result;
use crate::infrastructure::config::settings::Config;
use crate::infrastructure::exchange::factory::ExchangeFactory;
use crate::infrastructure::exchange::reconnecting::ReconnectingDataStream;
use crate::port::outbound::exchange::MarketDataStream;
use tracing::info;

/// Build and connect the market stream with optional pooling.
pub(crate) async fn create_connected_stream(
    config: &Config,
    token_ids: &[TokenId],
) -> Result<Box<dyn MarketDataStream>> {
    let mut data_stream: Box<dyn MarketDataStream> =
        if let Some(pool) = ExchangeFactory::create_connection_pool(config)? {
            info!(exchange = pool.exchange_name(), "Using connection pool");
            Box::new(pool)
        } else {
            info!("Using single connection");
            let inner = ExchangeFactory::create_data_stream(config);
            Box::new(ReconnectingDataStream::new(
                inner,
                config.reconnection.clone(),
            ))
        };

    data_stream.connect().await?;
    data_stream.subscribe(token_ids).await?;
    Ok(data_stream)
}
