use rust_decimal::Decimal;

use edgelord::core::domain::{Market, MarketId, OrderBook, Outcome, TokenId};
use edgelord::core::exchange::MarketEvent;

/// Generate `n` token IDs named `t0`, `t1`, ..., `t{n-1}`.
pub fn make_tokens(n: usize) -> Vec<TokenId> {
    (0..n).map(|i| TokenId::from(format!("t{i}"))).collect()
}

/// Create an [`OrderBookSnapshot`] event for a given token ID string.
pub fn snapshot_event(token: &str) -> MarketEvent {
    MarketEvent::OrderBookSnapshot {
        token_id: TokenId::from(token.to_string()),
        book: OrderBook::new(TokenId::from(token.to_string())),
    }
}

pub fn make_binary_market(
    id: &str,
    question: &str,
    yes_token: &str,
    no_token: &str,
    payout: Decimal,
) -> Market {
    let outcomes = vec![
        Outcome::new(TokenId::from(yes_token), "Yes"),
        Outcome::new(TokenId::from(no_token), "No"),
    ];
    Market::new(MarketId::from(id), question, outcomes, payout)
}

pub fn make_multi_market(
    id: &str,
    question: &str,
    outcomes: &[(&str, &str)],
    payout: Decimal,
) -> Market {
    let outcomes = outcomes
        .iter()
        .map(|(token, name)| Outcome::new(TokenId::from(*token), *name))
        .collect();
    Market::new(MarketId::from(id), question, outcomes, payout)
}
