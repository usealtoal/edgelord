use std::collections::VecDeque;

use async_trait::async_trait;
use edgelord::domain::TokenId;
use edgelord::error::Error;
use edgelord::port::{MarketDataStream, MarketEvent};

/// Deterministic test double for market data streaming.
#[derive(Debug, Default)]
pub struct ScriptedMarketDataStream {
    connect_results: VecDeque<Result<(), Error>>,
    subscribe_results: VecDeque<Result<(), Error>>,
    events: VecDeque<Option<MarketEvent>>,
    subscriptions: Vec<Vec<TokenId>>,
    connect_calls: usize,
    subscribe_calls: usize,
}

impl ScriptedMarketDataStream {
    pub fn push_event(&mut self, event: MarketEvent) {
        self.events.push_back(Some(event));
    }

    pub fn push_connected(&mut self) {
        self.push_event(MarketEvent::Connected);
    }

    pub fn subscribe_tokens(&mut self, token_ids: &[TokenId]) {
        self.subscriptions.push(token_ids.to_vec());
    }

    pub fn subscriptions(&self) -> &[Vec<TokenId>] {
        &self.subscriptions
    }
}

#[async_trait]
impl MarketDataStream for ScriptedMarketDataStream {
    async fn connect(&mut self) -> Result<(), Error> {
        self.connect_calls += 1;
        self.connect_results.pop_front().unwrap_or(Ok(()))
    }

    async fn subscribe(&mut self, token_ids: &[TokenId]) -> Result<(), Error> {
        self.subscribe_calls += 1;
        self.subscriptions.push(token_ids.to_vec());
        self.subscribe_results.pop_front().unwrap_or(Ok(()))
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        self.events.pop_front().flatten()
    }

    fn exchange_name(&self) -> &'static str {
        "fake"
    }
}
