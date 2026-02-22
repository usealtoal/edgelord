//! Statistics operator implementation.

use chrono::NaiveDate;

use crate::adapter::outbound::sqlite::report::SqliteReportReader;
use crate::domain::stats::StatsSummary;
use crate::error::Result;
use crate::port::inbound::operator::statistics::{
    DailyStatsRecord, StatisticsOperator, StrategyStatsRecord,
};
use crate::port::outbound::report::StatisticsReportReader;

use super::entry::Operator;

impl StatisticsOperator for Operator {
    fn load_summary(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<StatsSummary> {
        SqliteReportReader::new(database_url).load_summary(from, to)
    }

    fn load_strategy_breakdown(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<StrategyStatsRecord>> {
        let rows = SqliteReportReader::new(database_url).load_strategy_breakdown(from, to)?;
        Ok(rows
            .into_iter()
            .map(|row| StrategyStatsRecord {
                strategy: row.strategy,
                opportunities_detected: row.opportunities_detected,
                opportunities_executed: row.opportunities_executed,
                trades_opened: row.trades_opened,
                trades_closed: row.trades_closed,
                profit_realized: row.profit_realized,
                win_count: row.win_count,
                loss_count: row.loss_count,
            })
            .collect())
    }

    fn load_open_positions(&self, database_url: &str) -> Result<i64> {
        SqliteReportReader::new(database_url).load_open_positions()
    }

    fn load_daily_rows(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DailyStatsRecord>> {
        let rows = SqliteReportReader::new(database_url).load_daily_rows(from, to)?;
        Ok(rows
            .into_iter()
            .map(|row| DailyStatsRecord {
                date: row.date,
                opportunities_detected: row.opportunities_detected,
                opportunities_executed: row.opportunities_executed,
                opportunities_rejected: row.opportunities_rejected,
                trades_opened: row.trades_opened,
                trades_closed: row.trades_closed,
                profit_realized: row.profit_realized,
                loss_realized: row.loss_realized,
                win_count: row.win_count,
                loss_count: row.loss_count,
                total_volume: row.total_volume,
            })
            .collect())
    }

    fn export_daily_csv(
        &self,
        database_url: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<String> {
        SqliteReportReader::new(database_url).export_daily_csv(from, to)
    }

    fn prune_old_records(&self, database_url: &str, retention_days: u32) -> Result<()> {
        SqliteReportReader::new(database_url).prune_old_records(retention_days)
    }
}
