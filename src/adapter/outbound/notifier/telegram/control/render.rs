use crate::domain::relation::RelationKind;
use crate::port::inbound::runtime::RuntimePositionStatus;

use super::{format_uptime, TelegramControl};

impl TelegramControl {
    pub(super) fn status_text(&self) -> String {
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

    pub(super) fn health_text(&self) -> String {
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

    pub(super) fn positions_text(&self) -> String {
        let active = self.state.active_positions();

        let total_active = active.len();

        if active.is_empty() {
            return "💼 No active positions".to_string();
        }

        let mut response = format!("💼 Active Positions ({})\n\n", active.len());

        let display_count = active.len().min(self.position_display_limit);
        for (i, p) in active.iter().take(self.position_display_limit).enumerate() {
            let (status_emoji, status) = match p.status {
                RuntimePositionStatus::Open => ("🟢", "open"),
                RuntimePositionStatus::PartialFill => ("🟡", "partial"),
                RuntimePositionStatus::Closed => ("⚫", "closed"),
            };

            let market_id = &p.market_id;
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
                p.entry_cost,
                p.expected_profit
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

    pub(super) fn stats_text(&self) -> String {
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

    pub(super) fn pool_text(&self) -> String {
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

    pub(super) fn markets_text(&self) -> String {
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

        // Show cluster information if available.
        if let Some(view) = runtime.cluster_view() {
            let clusters = view.all_clusters();
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

                // Show up to 3 clusters with their markets.
                for (i, cluster) in clusters.iter().take(3).enumerate() {
                    response.push_str(&format!(
                        "\n📦 Cluster {} ({} markets)\n",
                        i + 1,
                        cluster.markets.len()
                    ));

                    // Show relation types in this cluster.
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

                    // Show market IDs (truncated).
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

    pub(super) fn version_text(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");

        // Try to get git info if available (set during build).
        let commit = option_env!("GIT_COMMIT_SHORT").unwrap_or("unknown");
        let build_date = option_env!("BUILD_DATE").unwrap_or("unknown");

        format!(
            "🔖 Version v{}\n\n\
            🔗 Commit: {}\n\
            📅 Built: {}",
            version, commit, build_date
        )
    }
}
