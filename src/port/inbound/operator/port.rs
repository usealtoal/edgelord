//! Unified operator capability surface for inbound adapters.
//!
//! Combines all operator-facing use-case traits into a single composite trait
//! that can be consumed by inbound adapters.

use super::config::ConfigurationOperator;
use super::diagnostic::DiagnosticOperator;
use super::runtime::RuntimeOperator;
use super::stats::StatisticsOperator;
use super::status::StatusOperator;
use super::wallet::WalletOperator;

/// Unified operator capability surface.
///
/// Combines all operator-facing use-case traits into a single interface
/// that inbound adapters (CLI, Telegram bot) can consume.
///
/// # Composition
///
/// This trait is automatically implemented for any type that implements
/// all constituent traits:
///
/// - [`ConfigurationOperator`]: Configuration display and validation
/// - [`DiagnosticOperator`]: Health checks and diagnostics
/// - [`RuntimeOperator`]: Runtime control and monitoring
/// - [`StatisticsOperator`]: Trading statistics queries
/// - [`StatusOperator`]: Current status snapshots
/// - [`WalletOperator`]: Wallet management
pub trait OperatorPort:
    ConfigurationOperator
    + DiagnosticOperator
    + RuntimeOperator
    + StatisticsOperator
    + StatusOperator
    + WalletOperator
{
}

/// Blanket implementation for types implementing all operator traits.
impl<T> OperatorPort for T where
    T: ConfigurationOperator
        + DiagnosticOperator
        + RuntimeOperator
        + StatisticsOperator
        + StatusOperator
        + WalletOperator
{
}
