//! Unified operator capability surface for inbound adapters.

use super::config::ConfigurationOperator;
use super::diagnostic::DiagnosticOperator;
use super::runtime::RuntimeOperator;
use super::stats::StatisticsOperator;
use super::status::StatusOperator;
use super::wallet::WalletOperator;

/// Unified operator capability surface consumed by inbound adapters.
pub trait OperatorPort:
    ConfigurationOperator
    + DiagnosticOperator
    + RuntimeOperator
    + StatisticsOperator
    + StatusOperator
    + WalletOperator
{
}

impl<T> OperatorPort for T where
    T: ConfigurationOperator
        + DiagnosticOperator
        + RuntimeOperator
        + StatisticsOperator
        + StatusOperator
        + WalletOperator
{
}
