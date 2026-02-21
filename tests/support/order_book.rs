use rust_decimal::Decimal;

use edgelord::domain::{OrderBook, PriceLevel, TokenId};
use edgelord::runtime::cache::OrderBookCache;

pub fn make_order_book(token_id: &str, bid: Decimal, ask: Decimal) -> OrderBook {
    OrderBook::with_levels(
        TokenId::from(token_id),
        vec![PriceLevel::new(bid, Decimal::new(100, 0))],
        vec![PriceLevel::new(ask, Decimal::new(100, 0))],
    )
}

pub fn set_order_book(cache: &OrderBookCache, token_id: &str, bid: Decimal, ask: Decimal) {
    cache.update(make_order_book(token_id, bid, ask));
}
