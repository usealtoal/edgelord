//! Connection pool management task.
//!
//! Provides the background task that monitors connection health and
//! performs automatic rotations and restarts.

use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use super::replace::{replace_connection, ManagementContext, ReplacementJob, ReplacementReason};
use super::state::{epoch_millis, lock_or_recover};
use super::MANAGEMENT_CONNECTION_ID_START;

/// Background task that monitors connection health and performs rotations.
///
/// Runs on a fixed interval defined by `health_check_interval_secs`. Detects
/// three conditions requiring replacement:
///
/// - **TTL expiry**: Connections approaching their lifetime limit
/// - **Silent death**: Connections that stopped receiving events
/// - **Task crash**: Connection tasks that terminated unexpectedly
///
/// Replacements are processed concurrently via `join_all` so one slow handoff
/// does not block others.
pub(super) async fn management_task(ctx: ManagementContext, exchange_name: &'static str) {
    let check_interval = Duration::from_secs(ctx.config.health_check_interval_secs);
    let ttl_threshold = Duration::from_secs(
        ctx.config
            .connection_ttl_secs
            .saturating_sub(ctx.config.preemptive_reconnect_secs),
    );
    let max_silent_ms = ctx.config.max_silent_secs * 1000;
    let handoff_timeout = Duration::from_secs(ctx.config.connection_ttl_secs.max(30));

    let mut interval = tokio::time::interval(check_interval);
    let mut next_id: u64 = MANAGEMENT_CONNECTION_ID_START;

    debug!(exchange = exchange_name, "Management task started");

    loop {
        interval.tick().await;
        let now = Instant::now();
        let now_ms = epoch_millis();

        // Phase 1: identify connections needing replacement (brief lock).
        let jobs: Vec<ReplacementJob> = {
            let conns = lock_or_recover(&ctx.connections);
            conns
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    if c.handle.is_finished() {
                        warn!(connection_id = c.id, "Task finished unexpectedly");
                        return Some(ReplacementJob {
                            index: i,
                            reason: ReplacementReason::Crashed,
                        });
                    }
                    if now.duration_since(c.spawned_at) >= ttl_threshold {
                        info!(
                            connection_id = c.id,
                            age_secs = now.duration_since(c.spawned_at).as_secs(),
                            "Approaching TTL"
                        );
                        return Some(ReplacementJob {
                            index: i,
                            reason: ReplacementReason::Ttl,
                        });
                    }
                    let last = c.last_event_at.load(std::sync::atomic::Ordering::Relaxed);
                    if last > 0 && now_ms.saturating_sub(last) > max_silent_ms {
                        warn!(
                            connection_id = c.id,
                            silent_secs = now_ms.saturating_sub(last) / 1000,
                            "No events, appears dead"
                        );
                        return Some(ReplacementJob {
                            index: i,
                            reason: ReplacementReason::Silent,
                        });
                    }
                    None
                })
                .collect()
        };

        if jobs.is_empty() {
            continue;
        }

        // Phase 2: process replacements concurrently.
        let futures: Vec<_> = jobs
            .into_iter()
            .map(|job| {
                next_id += 1;
                replace_connection(&ctx, job.index, job.reason, next_id, handoff_timeout)
            })
            .collect();

        futures_util::future::join_all(futures).await;
    }
}
