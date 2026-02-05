//! Tests for exchange factory and approval components.

mod support;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use edgelord::app::{
    Config, ExchangeSpecificConfig, PolymarketConfig, PolymarketHttpConfig,
};
use edgelord::core::domain::OrderBook;
use edgelord::core::exchange::polymarket::PolymarketClient;
use edgelord::core::exchange::{MarketDataStream, MarketEvent, ReconnectingDataStream};
use edgelord::error::{ConfigError, Error};
use edgelord::core::exchange::ExchangeFactory;
use edgelord::core::domain::TokenId;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[test]
fn factory_returns_error_when_exchange_config_missing() {
    let mut config = Config::default();
    config.exchange_config = ExchangeSpecificConfig::Polymarket(PolymarketConfig {
        ws_url: String::new(),
        api_url: String::new(),
        ..Default::default()
    });

    let result = ExchangeFactory::create_scorer(&config);
    assert!(
        matches!(
            result,
            Err(Error::Config(ConfigError::MissingField { field: "ws_url" }))
        ),
        "Expected MissingField error when ws_url is empty"
    );

    let result = ExchangeFactory::create_filter(&config);
    assert!(
        matches!(
            result,
            Err(Error::Config(ConfigError::MissingField { field: "ws_url" }))
        ),
        "Expected MissingField error when ws_url is empty"
    );

    let result = ExchangeFactory::create_deduplicator(&config);
    assert!(
        matches!(
            result,
            Err(Error::Config(ConfigError::MissingField { field: "ws_url" }))
        ),
        "Expected MissingField error when ws_url is empty"
    );
}

struct MockDataStream {
    connect_results: VecDeque<Result<(), Error>>,
    subscribe_results: VecDeque<Result<(), Error>>,
    events: VecDeque<Option<MarketEvent>>,
    connect_count: Arc<AtomicU32>,
    subscribe_count: Arc<AtomicU32>,
}

impl MockDataStream {
    fn new() -> Self {
        Self {
            connect_results: VecDeque::new(),
            subscribe_results: VecDeque::new(),
            events: VecDeque::new(),
            connect_count: Arc::new(AtomicU32::new(0)),
            subscribe_count: Arc::new(AtomicU32::new(0)),
        }
    }

    fn with_connect_results(mut self, results: Vec<Result<(), Error>>) -> Self {
        self.connect_results = results.into();
        self
    }

    fn with_subscribe_results(mut self, results: Vec<Result<(), Error>>) -> Self {
        self.subscribe_results = results.into();
        self
    }

    fn with_events(mut self, events: Vec<Option<MarketEvent>>) -> Self {
        self.events = events.into();
        self
    }

    fn counts(&self) -> (Arc<AtomicU32>, Arc<AtomicU32>) {
        (Arc::clone(&self.connect_count), Arc::clone(&self.subscribe_count))
    }
}

#[async_trait::async_trait]
impl MarketDataStream for MockDataStream {
    async fn connect(&mut self) -> Result<(), Error> {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        self.connect_results.pop_front().unwrap_or(Ok(()))
    }

    async fn subscribe(&mut self, _token_ids: &[TokenId]) -> Result<(), Error> {
        self.subscribe_count.fetch_add(1, Ordering::SeqCst);
        self.subscribe_results.pop_front().unwrap_or(Ok(()))
    }

    async fn next_event(&mut self) -> Option<MarketEvent> {
        self.events.pop_front().flatten()
    }

    fn exchange_name(&self) -> &'static str {
        "mock"
    }
}

#[tokio::test]
async fn reconnect_retries_on_subscribe_failure() {
    let mock = MockDataStream::new()
        .with_connect_results(vec![Ok(()), Ok(()), Ok(())])
        .with_subscribe_results(vec![
            Ok(()),
            Err(Error::Connection("subscribe failed".into())),
            Ok(()),
        ])
        .with_events(vec![
            Some(MarketEvent::Disconnected {
                reason: "test disconnect".into(),
            }),
            Some(MarketEvent::OrderBookSnapshot {
                token_id: TokenId::from("token-1".to_string()),
                book: OrderBook::new(TokenId::from("token-1".to_string())),
            }),
        ]);
    let (connect_count, subscribe_count) = mock.counts();

    let mut stream = ReconnectingDataStream::new(mock, support::config::test_reconnection_config());
    stream.connect().await.unwrap();
    stream
        .subscribe(&[TokenId::from("token-1".to_string())])
        .await
        .unwrap();

    let event = stream.next_event().await;
    assert!(matches!(event, Some(MarketEvent::OrderBookSnapshot { .. })));
    assert!(
        connect_count.load(Ordering::SeqCst) >= 3,
        "Expected reconnect to retry after subscribe failure"
    );
    assert!(
        subscribe_count.load(Ordering::SeqCst) >= 3,
        "Expected resubscribe retry after failure"
    );
}

#[tokio::test]
async fn client_times_out_on_slow_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicU32::new(0));
    let accepted_clone = Arc::clone(&accepted);

    let server = tokio::spawn(async move {
        while accepted_clone.load(Ordering::SeqCst) < 2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            accepted_clone.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                tokio::time::sleep(Duration::from_millis(50)).await;
            });
        }
    });

    let base_url = format!("http://{}", addr);
    let config = PolymarketConfig {
        api_url: base_url,
        http: PolymarketHttpConfig {
            timeout_ms: 10,
            connect_timeout_ms: 10,
            retry_max_attempts: 2,
            retry_backoff_ms: 0,
        },
        ..Default::default()
    };

    let client = PolymarketClient::from_config(&config);
    let result = client.get_active_markets(1).await;

    assert!(
        matches!(result, Err(Error::Http(err)) if err.is_timeout()),
        "Expected timeout error for slow response"
    );
    assert!(
        accepted.load(Ordering::SeqCst) >= 2,
        "Expected retry attempts for slow response"
    );

    let _ = server.await;
}
