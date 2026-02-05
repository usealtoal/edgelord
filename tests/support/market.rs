use rust_decimal::Decimal;

use edgelord::core::domain::{Market, MarketId, Outcome, TokenId};

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
