//! Telegram command execution against runtime app state.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;

use crate::adapters::statistics::StatsRecorder;
use crate::domain::{PositionStatus, RelationKind};
use crate::runtime::PoolStats;
use crate::runtime::cache::ClusterCache;
use crate::runtime::AppState;

use super::command::{command_help, TelegramCommand};

/// Runtime statistics updated by the orchestrator.
///
/// These values are updated periodically and read by Telegram commands.
#[derive(Debug, Default)]
pub struct RuntimeStats {
    /// Connection pool statistics.
    pool_stats: RwLock<Option<PoolStats>>,
    /// Number of subscribed markets.
    market_count: AtomicUsize,
    /// Number of subscribed tokens.
    token_count: AtomicUsize,
    /// Cluster cache for relation lookups.
    cluster_cache: RwLock<Option<Arc<ClusterCache>>>,
}

impl RuntimeStats {
    /// Create a new runtime stats container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Update pool statistics.
    pub fn update_pool_stats(&self, stats: PoolStats) {
        *self.pool_stats.write() = Some(stats);
    }

    /// Update market and token counts.
    pub fn update_market_counts(&self, markets: usize, tokens: usize) {
        self.market_count.store(markets, Ordering::Relaxed);
        self.token_count.store(tokens, Ordering::Relaxed);
    }

    /// Get current pool stats.
    #[must_use]
    pub fn pool_stats(&self) -> Option<PoolStats> {
        self.pool_stats.read().clone()
    }

    /// Get market count.
    #[must_use]
    pub fn market_count(&self) -> usize {
        self.market_count.load(Ordering::Relaxed)
    }

    /// Get token count.
    #[must_use]
    pub fn token_count(&self) -> usize {
        self.token_count.load(Ordering::Relaxed)
    }

    /// Set cluster cache for relation lookups.
    pub fn set_cluster_cache(&self, cache: Arc<ClusterCache>) {
        *self.cluster_cache.write() = Some(cache);
    }

    /// Get cluster cache.
    #[must_use]
    pub fn cluster_cache(&self) -> Option<Arc<ClusterCache>> {
        self.cluster_cache.read().clone()
    }
}

/// Runtime command executor for Telegram control commands.
#[derive(Clone)]
pub struct TelegramControl {
    state: Arc<AppState>,
    stats_recorder: Option<Arc<StatsRecorder>>,
    runtime_stats: Option<Arc<RuntimeStats>>,
    started_at: chrono::DateTime<Utc>,
    /// Maximum positions to display in /positions command.
    position_display_limit: usize,
}

/// Default position display limit if not specified.
const DEFAULT_POSITION_DISPLAY_LIMIT: usize = 10;

impl TelegramControl {
    /// Create a new control with just app state (minimal).
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            stats_recorder: None,
            runtime_stats: None,
            started_at: Utc::now(),
            position_display_limit: DEFAULT_POSITION_DISPLAY_LIMIT,
        }
    }

    /// Create a control with full dependencies and custom position display limit.
    #[must_use]
    pub fn with_config(
        state: Arc<AppState>,
        stats_recorder: Arc<StatsRecorder>,
        runtime_stats: Arc<RuntimeStats>,
        position_display_limit: usize,
    ) -> Self {
        Self {
            state,
            stats_recorder: Some(stats_recorder),
            runtime_stats: Some(runtime_stats),
            started_at: Utc::now(),
            position_display_limit,
        }
    }

    /// Execute one parsed command and return response text.
    #[must_use]
    pub fn execute(&self, command: TelegramCommand) -> String {
        match command {
            TelegramCommand::Start | TelegramCommand::Help => command_help().to_string(),
            TelegramCommand::Status => self.status_text(),
            TelegramCommand::Health => self.health_text(),
            TelegramCommand::Positions => self.positions_text(),
            TelegramCommand::Stats => self.stats_text(),
            TelegramCommand::Pool => self.pool_text(),
            TelegramCommand::Markets => self.markets_text(),
            TelegramCommand::Version => self.version_text(),
            TelegramCommand::Pause => self.pause_text(),
            TelegramCommand::Resume => self.resume_text(),
            TelegramCommand::SetRisk { kind, value } => {
                match self.state.set_risk_limit(kind, value) {
                    Ok(limits) => format!(
                        "✅ Updated {} to {}\n\n\
                        ⚙️ Current limits:\n\
                        • 💰 min_profit: {}\n\
                        • 📉 max_slippage: {}\n\
                        • 📊 max_position: ${}\n\
                        • 💼 max_exposure: ${}",
                        kind.as_str(),
                        value,
                        limits.min_profit_threshold,
                        limits.max_slippage,
                        limits.max_position_per_market,
                        limits.max_total_exposure
                    ),
                    Err(err) => format!("❌ Error: cannot update {}: {}", kind.as_str(), err),
                }
            }
        }
    }

    fn status_text(&self) -> String {
        let limits = self.state.risk_limits();
        let open_positions = self.state.open_position_count();
        let exposure = self.state.total_exposure();
        let pending_exposure = self.state.pending_exposure();
        let pending_executions = self.state.pending_execution_count();
        let is_paused = self.state.is_circuit_breaker_active();

        let (mode_emoji, mode) = if is_paused {
            ("⏸️", "PAUSED")
        } else {
            ("▶️", "ACTIVE")
        };
        let breaker = if is_paused {
            self.state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "inactive".to_string()
        };

        format!(
            "📊 Status\n\n\
            {} Mode: {}\n\
            ⏱️ Uptime: {}\n\
            🛑 Circuit Breaker: {}\n\n\
            💼 Portfolio\n\
            • Open Positions: {}\n\
            • Exposure: ${}\n\
            • Pending: ${}\n\
            • In-Flight: {}\n\n\
            ⚙️ Risk Limits\n\
            • Min Profit: ${}\n\
            • Max Slippage: {}%\n\
            • Max Position: ${}\n\
            • Max Exposure: ${}",
            mode_emoji,
            mode,
            format_uptime(self.started_at),
            breaker,
            open_positions,
            exposure,
            pending_exposure,
            pending_executions,
            limits.min_profit_threshold,
            limits.max_slippage * rust_decimal::Decimal::from(100),
            limits.max_position_per_market,
            limits.max_total_exposure
        )
    }

    fn health_text(&self) -> String {
        let limits = self.state.risk_limits();
        let exposure = self.state.total_exposure();
        let pending_exposure = self.state.pending_exposure();
        let total_exposure = exposure + pending_exposure;
        let exposure_ok = total_exposure <= limits.max_total_exposure;
        let breaker_ok = !self.state.is_circuit_breaker_active();
        let slippage_ok = limits.max_slippage >= rust_decimal::Decimal::ZERO
            && limits.max_slippage <= rust_decimal::Decimal::ONE;

        let healthy = exposure_ok && breaker_ok && slippage_ok;
        let (status_emoji, status) = if healthy {
            ("✅", "HEALTHY")
        } else {
            ("⚠️", "DEGRADED")
        };

        let check = |ok: bool| if ok { "✅" } else { "❌" };

        let breaker_detail = if breaker_ok {
            "inactive".to_string()
        } else {
            self.state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "active".to_string())
        };

        format!(
            "🏥 Health Check: {} {}\n\n\
            🛑 Circuit Breaker: {} ({})\n\
            💰 Exposure: {} (${}/{})\n\
            📉 Slippage Config: {} ({})",
            status_emoji,
            status,
            check(breaker_ok),
            breaker_detail,
            check(exposure_ok),
            total_exposure,
            limits.max_total_exposure,
            check(slippage_ok),
            limits.max_slippage
        )
    }

    fn positions_text(&self) -> String {
        let positions = self.state.positions();
        let active: Vec<_> = positions
            .all()
            .filter(|p| !p.status().is_closed())
            .collect();

        let total_active = active.len();

        if active.is_empty() {
            return "💼 No active positions".to_string();
        }

        let mut response = format!("💼 Active Positions ({})\n\n", active.len());

        let display_count = active.len().min(self.position_display_limit);
        for (i, p) in active.iter().take(self.position_display_limit).enumerate() {
            let (status_emoji, status) = match p.status() {
                PositionStatus::Open => ("🟢", "open"),
                PositionStatus::PartialFill { .. } => ("🟡", "partial"),
                PositionStatus::Closed { .. } => ("⚫", "closed"),
            };

            let market_id = p.market_id().as_str();
            let market_display = if market_id.len() > 12 {
                format!("{}...", &market_id[..12])
            } else {
                market_id.to_string()
            };

            response.push_str(&format!(
                "{}. {} {} ({})\n   💵 Cost: ${} | 📈 Expected: +${}\n",
                i + 1,
                status_emoji,
                market_display,
                status,
                p.entry_cost(),
                p.expected_profit()
            ));
        }

        if total_active > display_count {
            response.push_str(&format!(
                "\n📋 ... and {} more",
                total_active - display_count
            ));
        }

        response
    }

    fn stats_text(&self) -> String {
        let Some(ref recorder) = self.stats_recorder else {
            return "📈 Statistics not available".to_string();
        };

        let summary = recorder.get_today();

        let win_rate = summary
            .win_rate()
            .map(|r| format!("{:.1}%", r))
            .unwrap_or_else(|| "N/A".to_string());

        let net = summary.net_profit();
        let net_emoji = if net >= rust_decimal::Decimal::ZERO {
            "📈"
        } else {
            "📉"
        };

        format!(
            "📊 Today's Statistics\n\n\
            🎯 Opportunities: {} detected, {} executed\n\
            📋 Trades: {} opened, {} closed\n\
            🏆 Win Rate: {} ({} wins, {} losses)\n\
            💵 Volume: ${}\n\n\
            💰 P&L\n\
            • ✅ Realized Profit: ${}\n\
            • ❌ Realized Loss: ${}\n\
            • {} Net: ${}",
            summary.opportunities_detected,
            summary.opportunities_executed,
            summary.trades_opened,
            summary.trades_closed,
            win_rate,
            summary.win_count,
            summary.loss_count,
            summary.total_volume,
            summary.profit_realized,
            summary.loss_realized,
            net_emoji,
            net
        )
    }

    fn pool_text(&self) -> String {
        let Some(ref runtime) = self.runtime_stats else {
            return "🔌 Pool statistics not available".to_string();
        };

        let Some(stats) = runtime.pool_stats() else {
            return "🔌 Pool not initialized".to_string();
        };

        format!(
            "🔌 Connection Pool\n\n\
            🟢 Active Connections: {}\n\
            🔄 TTL Rotations: {}\n\
            🔃 Restarts: {}\n\
            ⚠️ Events Dropped: {}",
            stats.active_connections,
            stats.total_rotations,
            stats.total_restarts,
            stats.events_dropped
        )
    }

    fn markets_text(&self) -> String {
        let Some(ref runtime) = self.runtime_stats else {
            return "🏛️ Market statistics not available".to_string();
        };

        let markets = runtime.market_count();
        let tokens = runtime.token_count();

        if markets == 0 && tokens == 0 {
            return "🏛️ No markets subscribed".to_string();
        }

        let mut response = format!(
            "🏛️ Subscribed Markets\n\n\
            📊 Markets: {}\n\
            🪙 Tokens: {}\n",
            markets, tokens
        );

        // Show cluster information if available
        if let Some(cache) = runtime.cluster_cache() {
            let clusters = cache.all_clusters();
            if !clusters.is_empty() {
                let total_clustered_markets: usize = clusters.iter().map(|c| c.markets.len()).sum();
                let total_relations: usize = clusters.iter().map(|c| c.relations.len()).sum();

                response.push_str(&format!(
                    "\n🔗 Related Market Clusters: {}\n\
                    📈 Markets in clusters: {}\n\
                    🔀 Discovered relations: {}\n",
                    clusters.len(),
                    total_clustered_markets,
                    total_relations
                ));

                // Show up to 3 clusters with their markets
                for (i, cluster) in clusters.iter().take(3).enumerate() {
                    response.push_str(&format!(
                        "\n📦 Cluster {} ({} markets)\n",
                        i + 1,
                        cluster.markets.len()
                    ));

                    // Show relation types in this cluster
                    let mut relation_types: Vec<&str> = cluster
                        .relations
                        .iter()
                        .map(|r| match &r.kind {
                            RelationKind::MutuallyExclusive { .. } => "🔀 Mutually Exclusive",
                            RelationKind::Implies { .. } => "➡️ Implies",
                            RelationKind::ExactlyOne { .. } => "☝️ Exactly One",
                            RelationKind::Linear { .. } => "📐 Linear",
                        })
                        .collect();
                    relation_types.dedup();
                    for rt in relation_types {
                        response.push_str(&format!("  {}\n", rt));
                    }

                    // Show market IDs (truncated)
                    for market_id in cluster.markets.iter().take(5) {
                        let id = market_id.as_str();
                        let display = if id.len() > 16 {
                            format!("{}...", &id[..16])
                        } else {
                            id.to_string()
                        };
                        response.push_str(&format!("  • {}\n", display));
                    }
                    if cluster.markets.len() > 5 {
                        response
                            .push_str(&format!("  ... and {} more\n", cluster.markets.len() - 5));
                    }
                }

                if clusters.len() > 3 {
                    response.push_str(&format!(
                        "\n📋 ... and {} more clusters",
                        clusters.len() - 3
                    ));
                }
            }
        }

        response
    }

    fn version_text(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");

        // Try to get git info if available (set during build)
        let commit = option_env!("GIT_COMMIT_SHORT").unwrap_or("unknown");
        let build_date = option_env!("BUILD_DATE").unwrap_or("unknown");

        format!(
            "🔖 Version v{}\n\n\
            🔗 Commit: {}\n\
            📅 Built: {}",
            version, commit, build_date
        )
    }

    fn pause_text(&self) -> String {
        if self.state.is_circuit_breaker_active() {
            let reason = self
                .state
                .circuit_breaker_reason()
                .unwrap_or_else(|| "unknown".to_string());
            return format!("⏸️ Already paused: {}", reason);
        }

        self.state.activate_circuit_breaker("paused via Telegram");
        "⏸️ Trading paused".to_string()
    }

    fn resume_text(&self) -> String {
        if !self.state.is_circuit_breaker_active() {
            return "▶️ Trading already active".to_string();
        }

        self.state.reset_circuit_breaker();
        "▶️ Trading resumed".to_string()
    }
}

fn format_uptime(started_at: chrono::DateTime<Utc>) -> String {
    let elapsed = Utc::now() - started_at;
    let total_seconds = elapsed.num_seconds().max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    use crate::adapters::statistics;
    use crate::adapters::stores::db;
    use crate::runtime::{RiskLimitKind, RiskLimits};

    #[test]
    fn execute_pause_and_resume() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(Arc::clone(&state));

        let paused = control.execute(TelegramCommand::Pause);
        assert!(paused.contains("paused"));
        assert!(state.is_circuit_breaker_active());

        let resumed = control.execute(TelegramCommand::Resume);
        assert!(resumed.contains("resumed"));
        assert!(!state.is_circuit_breaker_active());
    }

    #[test]
    fn execute_set_risk_valid() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(Arc::clone(&state));

        let text = control.execute(TelegramCommand::SetRisk {
            kind: RiskLimitKind::MinProfitThreshold,
            value: dec!(0.4),
        });

        assert!(text.contains("Updated min_profit"));
        assert_eq!(state.risk_limits().min_profit_threshold, dec!(0.4));
    }

    #[test]
    fn execute_set_risk_invalid() {
        let state = Arc::new(AppState::new(RiskLimits::default()));
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::SetRisk {
            kind: RiskLimitKind::MaxSlippage,
            value: dec!(2),
        });

        assert!(text.contains("Error"));
        assert!(text.contains("max_slippage"));
    }

    #[test]
    fn execute_positions_empty() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Positions);
        assert!(text.contains("No active positions"));
    }

    #[test]
    fn execute_version() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Version);
        assert!(text.contains("Version"));
        assert!(text.contains("v0."));
    }

    #[test]
    fn execute_stats_without_recorder() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Stats);
        assert!(text.contains("not available"));
    }

    #[test]
    fn execute_pool_without_runtime_stats() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Pool);
        assert!(text.contains("not available"));
    }

    #[test]
    fn runtime_stats_update_and_read() {
        let stats = RuntimeStats::new();

        stats.update_market_counts(10, 20);
        assert_eq!(stats.market_count(), 10);
        assert_eq!(stats.token_count(), 20);

        stats.update_pool_stats(PoolStats {
            active_connections: 3,
            total_rotations: 5,
            total_restarts: 1,
            events_dropped: 0,
        });

        let pool = stats.pool_stats().unwrap();
        assert_eq!(pool.active_connections, 3);
        assert_eq!(pool.total_rotations, 5);
    }

    #[test]
    fn execute_stats_with_recorder() {
        // Create in-memory database for test
        let pool = db::create_pool("sqlite://:memory:").expect("create pool");
        db::run_migrations(&pool).expect("run migrations");
        let recorder = statistics::create_recorder(pool);

        let state = Arc::new(AppState::default());
        let runtime = Arc::new(RuntimeStats::new());
        let control = TelegramControl::with_config(state, recorder, runtime, 10);

        let text = control.execute(TelegramCommand::Stats);
        assert!(text.contains("Today's Statistics"));
        assert!(text.contains("Opportunities:"));
        assert!(text.contains("P&L"));
    }

    #[test]
    fn execute_pool_with_stats() {
        let state = Arc::new(AppState::default());
        let pool = db::create_pool("sqlite://:memory:").expect("create pool");
        db::run_migrations(&pool).expect("run migrations");
        let recorder = statistics::create_recorder(pool);
        let runtime = Arc::new(RuntimeStats::new());

        // Update pool stats
        runtime.update_pool_stats(PoolStats {
            active_connections: 5,
            total_rotations: 10,
            total_restarts: 2,
            events_dropped: 100,
        });

        let control = TelegramControl::with_config(state, recorder, runtime, 10);

        let text = control.execute(TelegramCommand::Pool);
        assert!(text.contains("Connection Pool"));
        assert!(text.contains("Active Connections: 5"));
        assert!(text.contains("TTL Rotations: 10"));
        assert!(text.contains("Events Dropped: 100"));
    }

    #[test]
    fn execute_markets_with_stats() {
        let state = Arc::new(AppState::default());
        let pool = db::create_pool("sqlite://:memory:").expect("create pool");
        db::run_migrations(&pool).expect("run migrations");
        let recorder = statistics::create_recorder(pool);
        let runtime = Arc::new(RuntimeStats::new());

        // Update market counts
        runtime.update_market_counts(42, 84);

        let control = TelegramControl::with_config(state, recorder, runtime, 10);

        let text = control.execute(TelegramCommand::Markets);
        assert!(text.contains("Subscribed Markets"));
        assert!(text.contains("Markets: 42"));
        assert!(text.contains("Tokens: 84"));
    }

    #[test]
    fn execute_markets_without_runtime_stats() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Markets);
        assert!(text.contains("not available"));
    }

    #[test]
    fn execute_markets_with_zero_counts() {
        let state = Arc::new(AppState::default());
        let pool = db::create_pool("sqlite://:memory:").expect("create pool");
        db::run_migrations(&pool).expect("run migrations");
        let recorder = statistics::create_recorder(pool);
        let runtime = Arc::new(RuntimeStats::new());
        // Don't update counts - they default to 0

        let control = TelegramControl::with_config(state, recorder, runtime, 10);

        let text = control.execute(TelegramCommand::Markets);
        assert!(text.contains("No markets subscribed"));
    }

    #[test]
    fn execute_pool_not_initialized() {
        let state = Arc::new(AppState::default());
        let pool = db::create_pool("sqlite://:memory:").expect("create pool");
        db::run_migrations(&pool).expect("run migrations");
        let recorder = statistics::create_recorder(pool);
        let runtime = Arc::new(RuntimeStats::new());
        // Don't update pool stats

        let control = TelegramControl::with_config(state, recorder, runtime, 10);

        let text = control.execute(TelegramCommand::Pool);
        assert!(text.contains("Pool not initialized"));
    }

    #[test]
    fn execute_help_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Help);
        assert!(text.contains("/status"));
        assert!(text.contains("/health"));
    }

    #[test]
    fn execute_start_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Start);
        assert!(text.contains("/status"));
        assert!(text.contains("/positions"));
        assert!(text.contains("Commands"));
    }

    #[test]
    fn execute_status_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Status);
        assert!(text.contains("Status"));
        assert!(text.contains("Mode:"));
        assert!(text.contains("Risk Limits"));
        assert!(text.contains("Portfolio"));
    }

    #[test]
    fn execute_health_command() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Health);
        assert!(text.contains("Health Check:"));
        assert!(text.contains("Circuit Breaker:"));
        assert!(text.contains("Exposure:"));
    }

    #[test]
    fn pause_when_already_paused() {
        let state = Arc::new(AppState::default());
        state.activate_circuit_breaker("manual pause");
        let control = TelegramControl::new(Arc::clone(&state));

        let text = control.execute(TelegramCommand::Pause);
        assert!(text.contains("Already paused"));
    }

    #[test]
    fn resume_when_not_paused() {
        let state = Arc::new(AppState::default());
        let control = TelegramControl::new(state);

        let text = control.execute(TelegramCommand::Resume);
        assert!(text.contains("Trading already active"));
    }

    #[test]
    fn positions_with_data() {
        use crate::domain::{MarketId, Position, PositionLeg, TokenId};

        let state = Arc::new(AppState::default());

        // Add a position
        {
            let mut positions = state.positions_mut();
            let legs = vec![
                PositionLeg::new(TokenId::from("yes-1"), dec!(100), dec!(0.40)),
                PositionLeg::new(TokenId::from("no-1"), dec!(100), dec!(0.50)),
            ];
            let position = Position::new(
                positions.next_id(),
                MarketId::from("test-market-12345"),
                legs,
                dec!(90),
                dec!(100),
                chrono::Utc::now(),
                PositionStatus::Open,
            );
            positions.add(position);
        }

        let control = TelegramControl::new(Arc::clone(&state));
        let text = control.execute(TelegramCommand::Positions);

        assert!(text.contains("Active Positions (1)"));
        assert!(text.contains("test-market-"));
        assert!(text.contains("open"));
    }

    #[test]
    fn format_uptime_test() {
        use chrono::{Duration, Utc};

        let now = Utc::now();
        let started = now - Duration::hours(2) - Duration::minutes(30) - Duration::seconds(45);

        let uptime = format_uptime(started);
        assert!(uptime.contains("02:30:45") || uptime.contains("02:30:46")); // Allow for timing
    }

    #[test]
    fn runtime_stats_pool_stats_none_initially() {
        let stats = RuntimeStats::new();
        assert!(stats.pool_stats().is_none());
    }

    #[test]
    fn status_shows_paused_when_circuit_breaker_active() {
        let state = Arc::new(AppState::default());
        state.activate_circuit_breaker("test reason");
        let control = TelegramControl::new(Arc::clone(&state));

        let text = control.execute(TelegramCommand::Status);
        assert!(text.contains("PAUSED"));
        assert!(text.contains("test reason"));
    }

    #[test]
    fn health_shows_degraded_when_circuit_breaker_active() {
        let state = Arc::new(AppState::default());
        state.activate_circuit_breaker("test failure");
        let control = TelegramControl::new(Arc::clone(&state));

        let text = control.execute(TelegramCommand::Health);
        assert!(text.contains("DEGRADED"));
        assert!(text.contains("❌")); // Changed from "FAIL" to emoji
    }
}
