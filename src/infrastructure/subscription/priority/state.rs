use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::domain::id::{MarketId, TokenId};
use crate::error::{Error, Result};

use super::PrioritySubscriptionManager;

pub(super) fn read_lock<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<'_, T>> {
    lock.read()
        .map_err(|_| Error::Connection("lock poisoned".to_string()))
}

pub(super) fn write_lock<T>(lock: &RwLock<T>) -> Result<RwLockWriteGuard<'_, T>> {
    lock.write()
        .map_err(|_| Error::Connection("lock poisoned".to_string()))
}

pub(super) fn read_lock_or_recover<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|err| err.into_inner())
}

pub(super) fn write_lock_or_recover<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|err| err.into_inner())
}

impl PrioritySubscriptionManager {
    pub(super) fn active_tokens_snapshot(&self) -> Vec<TokenId> {
        let active_tokens = read_lock_or_recover(&self.active_tokens);
        active_tokens.clone()
    }

    pub(super) fn active_tokens_count(&self) -> usize {
        let active_tokens = read_lock_or_recover(&self.active_tokens);
        active_tokens.len()
    }

    pub(super) fn pending_markets_count(&self) -> usize {
        let pending = read_lock_or_recover(&self.pending);
        pending.len()
    }

    pub(super) fn is_market_subscribed(&self, market_id: &MarketId) -> bool {
        let active_markets = read_lock_or_recover(&self.active_markets);
        active_markets.contains(market_id)
    }
}
