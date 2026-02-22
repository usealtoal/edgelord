//! Status operator implementation.

use crate::adapter::outbound::sqlite::report::SqliteReportReader;
use crate::error::Result;
use crate::infrastructure::config;
use crate::port::inbound::operator::status::{
    DailyStatusSummary, RecentActivity, StatusOperator, StatusSnapshot,
};
use crate::port::outbound::report::RecentActivity as ReportRecentActivity;
use crate::port::outbound::report::StatusReportReader;

use super::{entry::Operator, shared};

impl StatusOperator for Operator {
    fn network_label(&self, config_toml: &str) -> Result<String> {
        let config = config::settings::Config::parse_toml(config_toml)?;
        Ok(shared::network_label(
            config.network().environment,
            config.network().chain_id,
        ))
    }

    fn load_status(&self, database_url: &str) -> Result<StatusSnapshot> {
        let snapshot = SqliteReportReader::new(database_url).load_status()?;
        let today = snapshot.today.map(|row| DailyStatusSummary {
            opportunities_detected: row.opportunities_detected,
            opportunities_executed: row.opportunities_executed,
            opportunities_rejected: row.opportunities_rejected,
            profit_realized: row.profit_realized,
            loss_realized: row.loss_realized,
        });
        let recent_activity = snapshot
            .recent_activity
            .into_iter()
            .map(|item| match item {
                ReportRecentActivity::Executed {
                    timestamp,
                    profit,
                    market_description,
                } => RecentActivity::Executed {
                    timestamp,
                    profit,
                    market_description,
                },
                ReportRecentActivity::Rejected { timestamp, reason } => {
                    RecentActivity::Rejected { timestamp, reason }
                }
            })
            .collect();

        Ok(StatusSnapshot {
            today,
            open_positions: snapshot.open_positions,
            distinct_markets: snapshot.distinct_markets,
            current_exposure: snapshot.current_exposure,
            recent_activity,
        })
    }
}
