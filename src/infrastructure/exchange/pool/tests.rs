use super::*;
use std::sync::atomic::AtomicU32;

use crate::testkit;
use crate::testkit::stream::{CyclingStream, OneEventThenSilentStream, ScriptedStream};

// -- Helpers --------------------------------------------------------------

/// Wraps [`testkit::domain::snapshot_event`] in `Option` for use with
/// event lists.
fn snapshot_event(token: &str) -> Option<MarketEvent> {
    Some(testkit::domain::snapshot_event(token))
}

/// Factory that creates mock streams sharing a connect counter.
///
/// When `cycle` is true, uses [`CyclingStream`] (events repeat forever).
/// When false, uses [`ScriptedStream`] (events delivered once, then stream
/// ends — useful for crash/silence detection tests).
fn counting_factory(
    connect_count: Arc<AtomicU32>,
    events: Vec<Option<MarketEvent>>,
    cycle: bool,
) -> StreamFactory {
    Arc::new(move || {
        let cc = Arc::clone(&connect_count);
        if cycle {
            let evts: Vec<MarketEvent> = events.iter().filter_map(|e| e.clone()).collect();
            Box::new(CyclingStream::new(evts, Duration::from_millis(10), cc))
        } else {
            let mut s = ScriptedStream::new().with_events(events.clone());
            s.set_connect_count(cc);
            Box::new(s)
        }
    })
}

// -- Config validation ----------------------------------------------------

#[test]
fn test_config_rejects_zero_ttl() {
    let mut cfg = testkit::config::pool(10, 500);
    cfg.connection_ttl_secs = 0;
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").is_err());
}

#[test]
fn test_config_rejects_preemptive_gte_ttl() {
    let mut cfg = testkit::config::pool(10, 500);
    cfg.preemptive_reconnect_secs = 120;
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").is_err());
}

#[test]
fn test_config_rejects_zero_max_connections() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(
        testkit::config::pool(0, 500),
        testkit::config::reconnection(),
        f,
        "t"
    )
    .is_err());
}

#[test]
fn test_config_rejects_zero_subs_per_conn() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(
        testkit::config::pool(10, 0),
        testkit::config::reconnection(),
        f,
        "t"
    )
    .is_err());
}

#[test]
fn test_config_rejects_zero_channel_capacity() {
    let mut cfg = testkit::config::pool(10, 500);
    cfg.channel_capacity = 0;
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").is_err());
}

#[test]
fn test_config_accepts_valid() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    assert!(ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t"
    )
    .is_ok());
}

// -- Distribution ---------------------------------------------------------

#[tokio::test]
async fn test_single_connection() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, vec![], false);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(10))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conns = lock_or_recover(&pool.connections);
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].tokens.len(), 10);
}

#[tokio::test]
async fn test_multiple_connections() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, vec![], false);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(1000))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conns = lock_or_recover(&pool.connections);
    assert_eq!(conns.len(), 2);
    assert_eq!(conns[0].tokens.len(), 500);
    assert_eq!(conns[1].tokens.len(), 500);
}

#[tokio::test]
async fn test_respects_max_connections() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, vec![], false);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(3, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(5000))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conns = lock_or_recover(&pool.connections);
    assert_eq!(conns.len(), 3);
    let total: usize = conns.iter().map(|c| c.tokens.len()).sum();
    assert_eq!(total, 5000);
}

#[tokio::test]
async fn test_distributes_evenly() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, vec![], false);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(1250))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conns = lock_or_recover(&pool.connections);
    assert_eq!(conns.len(), 3);
    assert_eq!(conns[0].tokens.len(), 500);
    assert_eq!(conns[1].tokens.len(), 500);
    assert_eq!(conns[2].tokens.len(), 250);
}

// -- Event merging --------------------------------------------------------

#[tokio::test]
async fn test_merges_events() {
    let events = vec![snapshot_event("t1"), snapshot_event("t2")];
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, events, false);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(matches!(
        pool.next_event().await,
        Some(MarketEvent::BookSnapshot { .. })
    ));
    assert!(matches!(
        pool.next_event().await,
        Some(MarketEvent::BookSnapshot { .. })
    ));
}

// -- Identity / edge cases ------------------------------------------------

#[tokio::test]
async fn test_exchange_name() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    let cfg = testkit::config::pool(10, 500);

    let p1 = ConnectionPool::new(
        cfg.clone(),
        testkit::config::reconnection(),
        f.clone(),
        "polymarket",
    )
    .unwrap();
    assert_eq!(p1.exchange_name(), "polymarket");

    let p2 = ConnectionPool::new(cfg, testkit::config::reconnection(), f, "kalshi").unwrap();
    assert_eq!(p2.exchange_name(), "kalshi");
}

#[tokio::test]
async fn test_connect_is_noop() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    assert!(pool.connect().await.is_ok());
    assert!(lock_or_recover(&pool.connections).is_empty());
}

#[tokio::test]
async fn test_empty_subscribe() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    assert!(pool.subscribe(&[]).await.is_ok());
    assert!(lock_or_recover(&pool.connections).is_empty());
}

#[tokio::test]
async fn test_resubscribe_tears_down_old() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc, vec![snapshot_event("t1")], true);
    let mut pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();

    pool.subscribe(&testkit::domain::make_tokens(5))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(lock_or_recover(&pool.connections).len(), 1);

    pool.subscribe(&testkit::domain::make_tokens(10))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let conns = lock_or_recover(&pool.connections);
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].tokens.len(), 10);
}

// -- Stats ----------------------------------------------------------------

#[tokio::test]
async fn test_stats_initial() {
    let f: StreamFactory = Arc::new(|| Box::new(ScriptedStream::new()));
    let pool = ConnectionPool::new(
        testkit::config::pool(10, 500),
        testkit::config::reconnection(),
        f,
        "t",
    )
    .unwrap();
    let s = pool.stats();
    assert_eq!(s.active_connections, 0);
    assert_eq!(s.total_rotations, 0);
    assert_eq!(s.total_restarts, 0);
    assert_eq!(s.events_dropped, 0);
}

// -- Health monitoring ----------------------------------------------------

#[tokio::test]
async fn test_ttl_rotation() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

    let mut cfg = testkit::config::pool(10, 500);
    cfg.connection_ttl_secs = 2;
    cfg.preemptive_reconnect_secs = 1;
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").unwrap();
    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(4)).await;
    assert!(cc.load(Ordering::SeqCst) > 1, "Expected TTL rotation");
    assert!(pool.stats().total_rotations > 0);
}

#[tokio::test]
async fn test_preemptive_reconnect() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

    let mut cfg = testkit::config::pool(10, 500);
    cfg.connection_ttl_secs = 4;
    cfg.preemptive_reconnect_secs = 3;
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").unwrap();
    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        cc.load(Ordering::SeqCst) > 1,
        "Expected preemptive reconnect"
    );
}

#[tokio::test]
async fn test_silent_death_detection() {
    // Each stream delivers one event then blocks forever (alive but silent).
    // After max_silent_secs, the pool should detect silence and replace it.
    // The replacement also delivers one event (enabling handoff) then goes silent.
    let cc = Arc::new(AtomicU32::new(0));
    let factory: StreamFactory = {
        let cc = cc.clone();
        Arc::new(move || {
            Box::new(OneEventThenSilentStream::new(
                testkit::domain::snapshot_event("t1"),
                cc.clone(),
            )) as Box<dyn MarketDataStream>
        })
    };

    let mut cfg = testkit::config::pool(10, 500);
    cfg.max_silent_secs = 1;
    cfg.health_check_interval_secs = 1;
    cfg.connection_ttl_secs = 120;
    cfg.preemptive_reconnect_secs = 30;

    let mut pool = ConnectionPool::new(cfg, testkit::config::reconnection(), factory, "t").unwrap();
    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(
        cc.load(Ordering::SeqCst) > 1,
        "Expected restart after silence"
    );
    assert!(pool.stats().total_restarts > 0);
}

#[tokio::test]
async fn test_crashed_task_restart() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc.clone(), vec![], false);

    let mut cfg = testkit::config::pool(10, 500);
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").unwrap();
    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(
        cc.load(Ordering::SeqCst) > 1,
        "Expected crashed task restart"
    );
}

#[tokio::test]
async fn test_healthy_connection_not_replaced() {
    let cc = Arc::new(AtomicU32::new(0));
    let f = counting_factory(cc.clone(), vec![snapshot_event("t1")], true);

    let mut cfg = testkit::config::pool(10, 500);
    cfg.connection_ttl_secs = 120;
    cfg.max_silent_secs = 60;
    cfg.health_check_interval_secs = 1;

    let mut pool = ConnectionPool::new(cfg, testkit::config::reconnection(), f, "t").unwrap();
    pool.subscribe(&testkit::domain::make_tokens(1))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        cc.load(Ordering::SeqCst),
        1,
        "Healthy connection should not be replaced"
    );
}
