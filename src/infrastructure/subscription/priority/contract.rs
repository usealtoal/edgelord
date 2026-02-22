use std::collections::HashSet;

use tracing::{debug, info};

use crate::domain::id::TokenId;
use crate::error::Result;

use super::state::{read_lock, write_lock};
use super::PrioritySubscriptionManager;

impl PrioritySubscriptionManager {
    pub(super) async fn contract_tokens(&self, count: usize) -> Result<Vec<TokenId>> {
        let mut active_tokens = write_lock(&self.active_tokens)?;
        let mut active_markets = write_lock(&self.active_markets)?;
        let market_tokens = read_lock(&self.market_tokens)?;

        let mut removed_tokens = Vec::new();
        let mut tokens_to_remove = count.min(active_tokens.len());

        // LIFO removal: remove from the end of the active tokens list.
        while tokens_to_remove > 0 && !active_tokens.is_empty() {
            if let Some(token) = active_tokens.pop() {
                removed_tokens.push(token);
                tokens_to_remove -= 1;
            }
        }

        // Update active_markets by checking which markets no longer have active tokens.
        let active_token_set: HashSet<_> = active_tokens.iter().collect();

        // Find markets that no longer have any active tokens.
        let markets_to_remove: Vec<_> = active_markets
            .iter()
            .filter(|market_id| {
                if let Some(tokens) = market_tokens.get(*market_id) {
                    !tokens.iter().any(|t| active_token_set.contains(t))
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        for market_id in &markets_to_remove {
            debug!(
                market_id = %market_id,
                "Removing market from active subscriptions"
            );
            active_markets.remove(market_id);
        }

        info!(
            removed_tokens = removed_tokens.len(),
            removed_markets = markets_to_remove.len(),
            "Contracted subscriptions"
        );

        Ok(removed_tokens)
    }
}
