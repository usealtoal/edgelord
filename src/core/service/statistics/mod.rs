mod convert;
mod recorder;
mod types;

pub use convert::{decimal_to_f32, f32_to_decimal};
pub use recorder::{create_recorder, StatsRecorder};
pub use types::{
    OpportunitySummary, RecordedOpportunity, StatsSummary, TradeCloseEvent, TradeLeg, TradeOpenEvent,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn stats_summary_win_rate_with_trades() {
        let summary = StatsSummary {
            win_count: 7,
            loss_count: 3,
            ..Default::default()
        };
        assert!((summary.win_rate().unwrap() - 70.0).abs() < 0.001);
    }

    #[test]
    fn stats_summary_win_rate_no_trades() {
        let summary = StatsSummary::default();
        assert!(summary.win_rate().is_none());
    }

    #[test]
    fn stats_summary_net_profit() {
        let summary = StatsSummary {
            profit_realized: dec!(100),
            loss_realized: dec!(30),
            ..Default::default()
        };
        assert_eq!(summary.net_profit(), dec!(70));
    }

    #[test]
    fn decimal_conversion_roundtrip() {
        let d = dec!(123.45);
        let f = decimal_to_f32(d);
        let back = f32_to_decimal(f);
        // f32 precision loss is acceptable
        assert!((back - d).abs() < dec!(0.01));
    }
}
