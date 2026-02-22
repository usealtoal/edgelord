//! Detection context used by orchestration flow.

use rust_decimal::Decimal;

use crate::application::cache::book::BookCache;
use crate::domain::{book::Book, id::MarketId, id::TokenId, market::Market};
use crate::port::{inbound::strategy::DetectionContext, inbound::strategy::MarketContext};

/// Detection context for a single market snapshot.
pub struct MarketDetectionContext<'a> {
    market: &'a Market,
    cache: &'a BookCache,
    market_ctx: MarketContext,
}

impl<'a> MarketDetectionContext<'a> {
    /// Build context from market metadata and current book cache.
    #[must_use]
    pub fn new(market: &'a Market, cache: &'a BookCache) -> Self {
        let market_ctx = if market.is_binary() {
            MarketContext::binary()
        } else {
            MarketContext::multi_outcome(market.outcome_count())
        };

        Self {
            market,
            cache,
            market_ctx,
        }
    }
}

impl DetectionContext for MarketDetectionContext<'_> {
    fn market_id(&self) -> &MarketId {
        self.market.market_id()
    }

    fn question(&self) -> &str {
        self.market.question()
    }

    fn token_ids(&self) -> Vec<TokenId> {
        self.market.token_ids().into_iter().cloned().collect()
    }

    fn payout(&self) -> Decimal {
        self.market.payout()
    }

    fn market_context(&self) -> MarketContext {
        self.market_ctx.clone()
    }

    fn best_ask(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache
            .get(token_id)?
            .best_ask()
            .map(|level| level.price())
    }

    fn best_bid(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache
            .get(token_id)?
            .best_bid()
            .map(|level| level.price())
    }

    fn ask_volume(&self, token_id: &TokenId) -> Option<Decimal> {
        self.cache
            .get(token_id)?
            .best_ask()
            .map(|level| level.size())
    }

    fn order_book(&self, token_id: &TokenId) -> Option<Book> {
        self.cache.get(token_id)
    }

    fn market(&self) -> &Market {
        self.market
    }
}
