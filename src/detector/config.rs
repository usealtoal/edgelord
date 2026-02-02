use rust_decimal::Decimal;
use serde::Deserialize;

/// Configuration for the arbitrage detector
#[derive(Debug, Clone, Deserialize)]
pub struct DetectorConfig {
    /// Minimum edge (profit per $1) to consider an opportunity
    /// e.g., 0.05 means YES + NO must be <= $0.95
    #[serde(default = "default_min_edge")]
    pub min_edge: Decimal,

    /// Minimum expected profit in dollars to act on
    #[serde(default = "default_min_profit")]
    pub min_profit: Decimal,
}

fn default_min_edge() -> Decimal {
    Decimal::new(5, 2) // 0.05
}

fn default_min_profit() -> Decimal {
    Decimal::new(50, 2) // 0.50
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_edge: default_min_edge(),
            min_profit: default_min_profit(),
        }
    }
}
