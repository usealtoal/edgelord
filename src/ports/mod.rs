//! Trait definitions (hexagonal ports). Depend only on domain.

mod exchange;
mod inference;
mod notifier;
mod risk;
mod solver;
mod store;
mod strategy;

pub use exchange::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher,
    MarketInfo, OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};
pub use inference::RelationInferrer;
pub use notifier::{Event, Notifier};
pub use risk::RiskGate;
pub use solver::Solver;
pub use store::Store;
pub use strategy::{DetectionContext, DetectionResult, MarketContext, Strategy};
