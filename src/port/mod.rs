//! Trait definitions (hexagonal ports). Depend only on domain.
//!
//! Ports define the extension points in the hexagonal architecture.
//! They are traits that adapters implement to integrate with external
//! systems (exchanges, databases, notification services, etc.).
//!
//! # Architecture
//!
//! ```text
//!                    ┌─────────────────────────┐
//!                    │      Application        │
//!                    │                         │
//!     ┌──────────────┤  Domain + Port          ├──────────────┐
//!     │              │                         │              │
//!     │              └─────────────────────────┘              │
//!     │                         │                             │
//!     ▼                         ▼                             ▼
//! ┌─────────┐            ┌─────────────┐              ┌───────────┐
//! │Exchange │            │   Store     │              │ Notifier  │
//! │ Adapter │            │   Adapter   │              │  Adapter  │
//! └─────────┘            └─────────────┘              └───────────┘
//! ```
//!
//! # Available Ports
//!
//! - [`MarketDataStream`], [`MarketFetcher`], [`OrderExecutor`], [`ArbitrageExecutor`] - Exchange integration
//! - [`Notifier`] - Event notifications (Telegram, logging, etc.)
//! - [`Store`] - Persistence for relations and clusters
//! - [`Solver`] - LP/ILP optimization backend
//! - [`RelationInferrer`] - Market relation discovery
//! - [`RiskGate`] - Trade validation and risk management
//!
//! Note: The `Strategy` trait is in `adapter::strategy` as it's an internal
//! extension point rather than an external dependency port.

mod exchange;
mod inference;
mod notifier;
mod risk;
mod solver;
mod store;
mod strategy;

// Exchange ports
pub use exchange::{
    ArbitrageExecutor, ExecutionResult, MarketDataStream, MarketEvent, MarketFetcher, MarketInfo,
    OrderExecutor, OrderRequest, OrderSide, OutcomeInfo,
};

// Inference port
pub use inference::{MarketSummary, RelationInferrer};

// Test utilities
#[cfg(test)]
pub use inference::tests;

// Notifier port
pub use notifier::{
    Event, ExecutionEvent, Notifier, OpportunityEvent, RelationDetail, RelationsEvent, RiskEvent,
    SummaryEvent,
};

// Risk port
pub use risk::{RiskCheckResult, RiskGate};

// Solver port
pub use solver::{
    Constraint, ConstraintSense, IlpProblem, LpProblem, LpSolution, SolutionStatus, Solver,
    VariableBounds,
};

// Store port
pub use store::Store;

// Strategy context types (Strategy trait is in adapter::strategy)
pub use strategy::{DetectionResult, MarketContext};
pub use strategy::DetectionContext;
