use tracing::{debug, info, warn};

use crate::domain::{id::TokenId, score::MarketScore};
use crate::error::Result;

use super::state::{read_lock, read_lock_or_recover, write_lock, write_lock_or_recover};
use super::PrioritySubscriptionManager;

impl PrioritySubscriptionManager {
    pub(super) fn enqueue_markets(&self, markets: Vec<MarketScore>) {
        let mut pending = write_lock_or_recover(&self.pending);
        let active_markets = read_lock_or_recover(&self.active_markets);

        for market in markets {
            // Skip markets that are already subscribed.
            if active_markets.contains(market.market_id()) {
                debug!(
                    market_id = %market.market_id(),
                    "Market already subscribed, skipping enqueue"
                );
                continue;
            }

            debug!(
                market_id = %market.market_id(),
                score = market.composite(),
                "Enqueueing market for subscription"
            );
            pending.push(market);
        }
    }

    pub(super) async fn expand_markets(&self, count: usize) -> Result<Vec<TokenId>> {
        let mut pending = write_lock(&self.pending)?;
        let mut active_markets = write_lock(&self.active_markets)?;
        let mut active_tokens = write_lock(&self.active_tokens)?;
        let market_tokens = read_lock(&self.market_tokens)?;

        let mut newly_subscribed = Vec::new();
        let mut markets_added = 0;

        // Keep popping from the queue until we've added enough tokens or run out.
        while markets_added < count {
            let Some(market_score) = pending.pop() else {
                debug!("No more markets in pending queue");
                break;
            };

            let market_id = market_score.market_id();

            // Skip if already active (could happen with duplicates in queue).
            if active_markets.contains(market_id) {
                continue;
            }

            // Get the tokens for this market.
            let Some(tokens) = market_tokens.get(market_id) else {
                warn!(
                    market_id = %market_id,
                    "No token mapping found for market, skipping"
                );
                continue;
            };

            // Check if we would exceed max subscriptions.
            let new_count = active_tokens.len() + tokens.len();
            if new_count > self.max_subscriptions {
                debug!(
                    market_id = %market_id,
                    current = active_tokens.len(),
                    adding = tokens.len(),
                    max = self.max_subscriptions,
                    "Would exceed max subscriptions, stopping expansion"
                );
                // Push the market back to the queue.
                pending.push(market_score);
                break;
            }

            info!(
                market_id = %market_id,
                score = market_score.composite(),
                token_count = tokens.len(),
                "Expanding subscription to market"
            );

            active_markets.insert(market_id.clone());
            for token in tokens {
                active_tokens.push(token.clone());
                newly_subscribed.push(token.clone());
            }
            markets_added += 1;
        }

        Ok(newly_subscribed)
    }
}
