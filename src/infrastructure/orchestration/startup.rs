//! Startup market discovery and strategy wiring.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::application::strategy::registry::StrategyRegistry;
use crate::domain::{id::TokenId, market::MarketRegistry};
use crate::error::Result;
use crate::infrastructure::config::settings::{Config, ExchangeSpecificConfig};
use crate::infrastructure::exchange::factory::ExchangeFactory;
use crate::port::inbound::strategy::StrategyEngine;
use crate::port::outbound::inference::MarketSummary;

/// Startup artifacts needed by the runtime event loop.
pub(crate) struct PreparedMarkets {
    pub registry: Arc<MarketRegistry>,
    pub strategies: Arc<StrategyRegistry>,
    pub token_ids: Vec<TokenId>,
    pub market_summaries: Vec<MarketSummary>,
}

/// Fetch, filter, parse, and wire markets into strategy runtime state.
pub(crate) async fn prepare_markets(
    config: &Config,
    mut strategies: StrategyRegistry,
) -> Result<Option<PreparedMarkets>> {
    info!(
        strategies = ?strategies.strategy_names(),
        "Strategies loaded"
    );

    let max_markets = match &config.exchange_config {
        ExchangeSpecificConfig::Polymarket(pm_config) => pm_config.market_filter.max_markets,
    };

    let market_fetcher = ExchangeFactory::create_market_fetcher(config);
    info!(
        exchange = market_fetcher.exchange_name(),
        max_markets, "Fetching markets"
    );
    let market_infos = market_fetcher.get_markets(max_markets).await?;
    let markets_fetched = market_infos.len();

    if market_infos.is_empty() {
        warn!("No active markets found");
        return Ok(None);
    }

    let market_filter = ExchangeFactory::create_filter(config)?;
    let market_infos = market_filter.filter(&market_infos);
    let markets_filtered = market_infos.len();

    info!(
        markets_fetched,
        markets_filtered,
        rejected = markets_fetched - markets_filtered,
        "Volume/liquidity filter applied"
    );

    if market_infos.is_empty() {
        warn!("No markets passed volume/liquidity filter");
        return Ok(None);
    }

    let market_parser = ExchangeFactory::create_market_parser(config);
    let markets = market_parser.parse_markets(&market_infos);
    let markets_parsed = markets.len();

    let mut registry = MarketRegistry::new();
    for market in markets {
        registry.add(market);
    }

    info!(
        markets_fetched,
        markets_parsed,
        yes_no_pairs = registry.len(),
        "Market scan complete"
    );

    if registry.is_empty() {
        warn!("No YES/NO market pairs found");
        return Ok(None);
    }

    for market in registry.markets() {
        debug!(
            market_id = %market.market_id(),
            question = %market.question(),
            "Tracking market"
        );
    }

    let market_summaries: Vec<MarketSummary> = registry
        .markets()
        .iter()
        .map(|m| MarketSummary {
            id: m.market_id().clone(),
            question: m.question().to_string(),
            outcomes: m.outcomes().iter().map(|o| o.name().to_string()).collect(),
        })
        .collect();

    let token_ids: Vec<TokenId> = registry
        .markets()
        .iter()
        .flat_map(|m| m.outcomes().iter().map(|o| o.token_id().clone()))
        .collect();

    info!(tokens = token_ids.len(), "Subscribing to tokens");

    let registry = Arc::new(registry);
    strategies.set_registry(Arc::clone(&registry));
    let strategies = Arc::new(strategies);

    Ok(Some(PreparedMarkets {
        registry,
        strategies,
        token_ids,
        market_summaries,
    }))
}
