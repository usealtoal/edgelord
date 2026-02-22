use rust_decimal::Decimal;

use edgelord::domain::{Book, PriceLevel, TokenId};
use edgelord::infrastructure::cache::BookCache;

pub fn make_book(token_id: &str, bid: Decimal, ask: Decimal) -> Book {
    Book::with_levels(
        TokenId::from(token_id),
        vec![PriceLevel::new(bid, Decimal::new(100, 0))],
        vec![PriceLevel::new(ask, Decimal::new(100, 0))],
    )
}

pub fn set_book(cache: &BookCache, token_id: &str, bid: Decimal, ask: Decimal) {
    cache.update(make_book(token_id, bid, ask));
}
