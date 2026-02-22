//! Slippage calculations for opportunity handling.

use rust_decimal::Decimal;

use crate::application::cache::book::BookCache;
use crate::domain::opportunity::Opportunity;

/// Get the maximum slippage across all opportunity legs.
///
/// Returns `None` when current prices cannot be derived from cache/books.
pub(crate) fn get_max_slippage(opportunity: &Opportunity, cache: &BookCache) -> Option<Decimal> {
    let mut max_slippage = Decimal::ZERO;

    for leg in opportunity.legs() {
        let book = cache.get(leg.token_id())?;
        let current_price = book.best_ask()?.price();
        let expected_price = leg.ask_price();

        if expected_price == Decimal::ZERO {
            return None;
        }

        let slippage = ((current_price - expected_price).abs()) / expected_price;
        max_slippage = max_slippage.max(slippage);
    }

    Some(max_slippage)
}
